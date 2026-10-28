mod catalog;
mod graph;

use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::diagnostics::{Diagnostic, DiagnosticCode, DiagnosticStage, Severity};

use super::{
    ArtifactEnvelope, ArtifactFormat, ArtifactStage, CodecError, ContentFingerprint, DecodeLimits,
    Identifier, LanguageRevision, NativeRequirement, UnitIdentity, encode, load, publish_new,
};

pub use graph::ArtifactGraph;

pub const CATALOG_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogEntry {
    pub identity: UnitIdentity,
    pub fingerprint: ContentFingerprint,
    pub language: LanguageRevision,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActiveBinding {
    pub identity: UnitIdentity,
    pub fingerprint: ContentFingerprint,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LibraryCatalog {
    pub version: u32,
    pub schema_version: u32,
    pub compiler_compatibility: u32,
    pub library: Identifier,
    pub entries: Vec<CatalogEntry>,
    pub bindings: Vec<ActiveBinding>,
}

impl LibraryCatalog {
    pub fn lookup(&self, identity: &UnitIdentity) -> Option<&CatalogEntry> {
        let identity = identity.normalized();
        let binding = self
            .bindings
            .iter()
            .find(|binding| binding.identity.normalized() == identity)?;
        self.entries.iter().find(|entry| {
            entry.identity.normalized() == identity && entry.fingerprint == binding.fingerprint
        })
    }
}

#[derive(Clone, Debug)]
pub struct StoreOptions {
    pub native_requirements: Vec<NativeRequirement>,
    pub limits: DecodeLimits,
    pub max_graph_artifacts: usize,
    pub max_graph_bytes: u64,
}

impl Default for StoreOptions {
    fn default() -> Self {
        Self {
            native_requirements: Vec::new(),
            limits: DecodeLimits::default(),
            max_graph_artifacts: 1024,
            max_graph_bytes: 256 * 1024 * 1024,
        }
    }
}

#[derive(Debug)]
pub enum StoreError {
    Codec(CodecError),
    Io(std::io::Error),
    InvalidCatalog(String),
    ConcurrentUpdate,
    IncompleteAnalysis(UnitIdentity),
    Missing(UnitIdentity),
    Stale(UnitIdentity),
    NativeCompatibility,
    ContentMismatch,
    Cycle(UnitIdentity),
    VersionConflict(UnitIdentity),
    ResourceLimit,
}

impl fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Codec(error) => error.fmt(formatter),
            Self::Io(error) => write!(formatter, "artifact store I/O: {error}"),
            Self::InvalidCatalog(message) => {
                write!(formatter, "invalid library catalog: {message}")
            }
            Self::ConcurrentUpdate => {
                formatter.write_str("library catalog changed; reopen before publishing")
            }
            Self::IncompleteAnalysis(identity) => {
                write!(formatter, "process analysis is incomplete for {identity:?}")
            }
            Self::Missing(identity) => {
                write!(formatter, "missing stored unit/version {identity:?}")
            }
            Self::Stale(identity) => write!(formatter, "stored dependency is stale: {identity:?}"),
            Self::NativeCompatibility => {
                formatter.write_str("native declaration set or ABI is incompatible")
            }
            Self::ContentMismatch => {
                formatter.write_str("stored artifact identity/fingerprint does not match catalog")
            }
            Self::Cycle(identity) => {
                write!(formatter, "cyclic artifact dependencies at {identity:?}")
            }
            Self::VersionConflict(identity) => write!(
                formatter,
                "conflicting versions in artifact graph: {identity:?}"
            ),
            Self::ResourceLimit => formatter.write_str("artifact graph resource limit exceeded"),
        }
    }
}

impl Error for StoreError {}

impl StoreError {
    pub fn diagnostic(&self) -> Diagnostic {
        let code = match self {
            Self::Codec(error) => return error.diagnostic(),
            Self::Io(_) => DiagnosticCode::ArtifactIo,
            Self::NativeCompatibility | Self::Stale(_) | Self::InvalidCatalog(_) => {
                DiagnosticCode::ArtifactCompatibility
            }
            Self::ResourceLimit => DiagnosticCode::ArtifactLimit,
            _ => DiagnosticCode::ArtifactValidation,
        };
        Diagnostic::new(
            code,
            DiagnosticStage::Artifact,
            Severity::Error,
            self.to_string(),
        )
    }
}

impl From<CodecError> for StoreError {
    fn from(error: CodecError) -> Self {
        Self::Codec(error)
    }
}

impl From<std::io::Error> for StoreError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

pub struct LibraryStore {
    root: PathBuf,
    catalog: LibraryCatalog,
    snapshot: Option<ContentFingerprint>,
    options: StoreOptions,
}

impl LibraryStore {
    pub fn open(
        root: &Path,
        library: Identifier,
        options: StoreOptions,
    ) -> Result<Self, StoreError> {
        let library = library.normalized();
        let mut native = BTreeSet::new();
        for requirement in &options.native_requirements {
            if requirement.abi_version != super::NATIVE_ABI_VERSION
                || !native.insert((
                    requirement.library.normalized(),
                    requirement.package.normalized(),
                ))
            {
                return Err(StoreError::NativeCompatibility);
            }
        }
        let (catalog, snapshot) = catalog::read(root, &library, options.limits)?;
        Ok(Self {
            root: root.into(),
            catalog,
            snapshot,
            options,
        })
    }

    pub fn catalog(&self) -> &LibraryCatalog {
        &self.catalog
    }

    pub fn catalog_path(&self) -> Result<PathBuf, StoreError> {
        catalog::path(&self.root, &self.catalog.library)
    }

    pub fn artifact_path(&self, fingerprint: ContentFingerprint) -> PathBuf {
        self.root
            .join("objects")
            .join(format!("{fingerprint}.osva"))
    }

    fn native_compatible(&self, artifact: &ArtifactEnvelope) -> Result<(), StoreError> {
        for required in &artifact.native_requirements {
            if !self.options.native_requirements.iter().any(|available| {
                available.library.normalized() == required.library.normalized()
                    && available.package.normalized() == required.package.normalized()
                    && available.abi_version == required.abi_version
                    && available.declarations == required.declarations
            }) {
                return Err(StoreError::NativeCompatibility);
            }
        }
        Ok(())
    }

    fn read_exact(
        &self,
        identity: &UnitIdentity,
        fingerprint: ContentFingerprint,
    ) -> Result<ArtifactEnvelope, StoreError> {
        let (catalog, _) = catalog::read(
            &self.root,
            &identity.library.normalized(),
            self.options.limits,
        )?;
        let entry = catalog
            .entries
            .iter()
            .find(|entry| {
                entry.identity == identity.normalized() && entry.fingerprint == fingerprint
            })
            .ok_or_else(|| StoreError::Missing(identity.clone()))?;
        let path = self.artifact_path(fingerprint);
        let artifact = load(&path, Some(ArtifactStage::Analysis), self.options.limits)?;
        if matches!(&artifact.payload, super::ArtifactPayload::Analysis(analysis) if !analysis.pending_processes.is_empty())
        {
            return Err(StoreError::IncompleteAnalysis(artifact.identity.clone()));
        }
        if artifact.identity != identity.normalized()
            || artifact.fingerprint().map_err(CodecError::Validation)? != fingerprint
            || artifact.language != entry.language
        {
            return Err(StoreError::ContentMismatch);
        }
        self.native_compatible(&artifact)?;
        Ok(artifact)
    }

    pub fn publish(
        &mut self,
        artifact: &ArtifactEnvelope,
        format: ArtifactFormat,
    ) -> Result<ContentFingerprint, StoreError> {
        let artifact = artifact.canonicalized().map_err(CodecError::Validation)?;
        artifact
            .validate(ArtifactStage::Analysis)
            .map_err(CodecError::Validation)?;
        if matches!(&artifact.payload, super::ArtifactPayload::Analysis(analysis) if !analysis.pending_processes.is_empty())
        {
            return Err(StoreError::IncompleteAnalysis(artifact.identity.clone()));
        }
        if artifact.identity.library != self.catalog.library {
            return Err(StoreError::ContentMismatch);
        }
        self.native_compatible(&artifact)?;
        self.check_dependencies(&artifact)?;
        let fingerprint = artifact.fingerprint().map_err(CodecError::Validation)?;
        let bytes = encode(&artifact, format)?;
        fs::create_dir_all(self.root.join("objects"))?;
        fs::create_dir_all(self.root.join("libraries"))?;
        let lock_path = self.root.join("catalog.lock");
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(lock_path)?;
        lock.lock()?;
        let (_, current) = catalog::read(&self.root, &self.catalog.library, self.options.limits)?;
        if current != self.snapshot {
            return Err(StoreError::ConcurrentUpdate);
        }
        let path = self.artifact_path(fingerprint);
        match publish_new(&path, &bytes) {
            Ok(()) => {}
            Err(CodecError::Io(error)) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let existing = load(&path, Some(ArtifactStage::Analysis), self.options.limits)?;
                if existing.fingerprint().map_err(CodecError::Validation)? != fingerprint {
                    return Err(StoreError::ContentMismatch);
                }
            }
            Err(error) => return Err(error.into()),
        }
        let mut next = self.catalog.clone();
        if !next
            .entries
            .iter()
            .any(|entry| entry.identity == artifact.identity && entry.fingerprint == fingerprint)
        {
            next.entries.push(CatalogEntry {
                identity: artifact.identity.clone(),
                fingerprint,
                language: artifact.language,
            });
        }
        next.bindings
            .retain(|binding| binding.identity != artifact.identity);
        next.bindings.push(ActiveBinding {
            identity: artifact.identity,
            fingerprint,
        });
        catalog::validate(&mut next, &self.catalog.library)?;
        let bytes = catalog::encode(&next)?;
        let path = catalog::path(&self.root, &self.catalog.library)?;
        let parent = path
            .parent()
            .ok_or_else(|| StoreError::InvalidCatalog("catalog has no parent".into()))?;
        let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
        temporary.write_all(&bytes)?;
        temporary.as_file().sync_all()?;
        temporary.persist(&path).map_err(|error| error.error)?;
        self.snapshot = Some(ContentFingerprint::of_bytes(&bytes));
        self.catalog = next;
        Ok(fingerprint)
    }

    fn check_dependencies(&self, artifact: &ArtifactEnvelope) -> Result<(), StoreError> {
        let mut versions = BTreeMap::new();
        versions.insert(
            artifact.identity.clone(),
            artifact.fingerprint().map_err(CodecError::Validation)?,
        );
        let mut bytes = 0u64;
        for dependency in &artifact.dependencies {
            let graph = self.resolve_exact(&dependency.identity, dependency.fingerprint)?;
            for artifact in graph.artifacts {
                let fingerprint = artifact.fingerprint().map_err(CodecError::Validation)?;
                if let Some(previous) = versions.insert(artifact.identity.clone(), fingerprint) {
                    if previous != fingerprint {
                        return Err(StoreError::VersionConflict(artifact.identity));
                    }
                    continue;
                }
                if versions.len() > self.options.max_graph_artifacts {
                    return Err(StoreError::ResourceLimit);
                }
                let size = fs::metadata(self.artifact_path(fingerprint))?.len();
                bytes = bytes
                    .checked_add(size)
                    .filter(|bytes| *bytes <= self.options.max_graph_bytes)
                    .ok_or(StoreError::ResourceLimit)?;
            }
        }
        Ok(())
    }
}
