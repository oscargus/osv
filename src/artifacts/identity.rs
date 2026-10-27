use std::{fmt, path::Path};

use crate::{diagnostics::display_path, source::SourceFile};

use super::{
    ArtifactEnvelope, ArtifactError, ContentFingerprint, ScopeKind, SourceRecord, TypeDefinition,
};

impl ContentFingerprint {
    pub fn of_bytes(bytes: &[u8]) -> Self {
        Self(*blake3::hash(bytes).as_bytes())
    }
}

impl fmt::Display for ContentFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl SourceRecord {
    pub fn embedded(file: SourceFile) -> Result<Self, ArtifactError> {
        let bytes = file
            .bytes
            .as_ref()
            .ok_or(ArtifactError::MissingSourceBytes(file.id))?;
        let fingerprint = ContentFingerprint::of_bytes(bytes);
        Ok(Self { file, fingerprint })
    }
}

impl ArtifactEnvelope {
    pub fn canonicalized(&self) -> Result<Self, ArtifactError> {
        self.validate(self.stage())?;
        let mut artifact = self.clone();
        artifact.identity = artifact.identity.normalized();
        for source in &mut artifact.sources {
            source.file.display_name = display_path(Path::new(&source.file.display_name));
        }
        for unit in &mut artifact.units {
            unit.identity = unit.identity.normalized();
        }
        for scope in &mut artifact.scopes {
            if let ScopeKind::Library(library) = &mut scope.kind {
                *library = library.normalized();
            }
        }
        for record in &mut artifact.types {
            record.name = record.name.normalized();
            if let TypeDefinition::Enumeration { literals } = &mut record.definition {
                for literal in literals {
                    *literal = literal.normalized();
                }
            }
        }
        for symbol in &mut artifact.symbols {
            symbol.name = symbol.name.normalized();
        }
        for dependency in &mut artifact.dependencies {
            dependency.identity = dependency.identity.normalized();
        }
        for requirement in &mut artifact.native_requirements {
            requirement.library = requirement.library.normalized();
            requirement.package = requirement.package.normalized();
        }
        artifact.sources.sort_by_key(|source| source.file.id);
        artifact.units.sort_by_key(|unit| unit.id);
        artifact.scopes.sort_by_key(|scope| scope.id);
        artifact.types.sort_by_key(|record| record.id);
        artifact.symbols.sort_by_key(|symbol| symbol.id);
        artifact.dependencies.sort_by(|left, right| {
            (&left.identity, left.role, left.stage).cmp(&(&right.identity, right.role, right.stage))
        });
        artifact.native_requirements.sort_by(|left, right| {
            (&left.library, &left.package).cmp(&(&right.library, &right.package))
        });
        Ok(artifact)
    }

    pub fn fingerprint(&self) -> Result<ContentFingerprint, ArtifactError> {
        let mut artifact = self.canonicalized()?;
        artifact.compiler_version.clear();
        let bytes = postcard::to_allocvec(&artifact)
            .map_err(|error| ArtifactError::Encoding(error.to_string()))?;
        Ok(ContentFingerprint::of_bytes(&bytes))
    }
}
