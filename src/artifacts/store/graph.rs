use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
};

use crate::artifacts::{ArtifactEnvelope, ContentFingerprint, UnitIdentity};

use super::{LibraryStore, StoreError, catalog};

pub struct ArtifactGraph {
    pub root: ContentFingerprint,
    pub artifacts: Vec<ArtifactEnvelope>,
}

impl LibraryStore {
    pub fn resolve_current(&self, identity: &UnitIdentity) -> Result<ArtifactGraph, StoreError> {
        let (catalog, _) = catalog::read(
            &self.root,
            &identity.library.normalized(),
            self.options.limits,
        )?;
        let entry = catalog
            .lookup(identity)
            .ok_or_else(|| StoreError::Missing(identity.clone()))?;
        self.resolve_graph(identity, entry.fingerprint, true)
    }

    pub fn resolve_exact(
        &self,
        identity: &UnitIdentity,
        fingerprint: ContentFingerprint,
    ) -> Result<ArtifactGraph, StoreError> {
        self.resolve_graph(identity, fingerprint, false)
    }

    fn resolve_graph(
        &self,
        identity: &UnitIdentity,
        fingerprint: ContentFingerprint,
        current: bool,
    ) -> Result<ArtifactGraph, StoreError> {
        let mut pending = vec![(identity.normalized(), fingerprint, false)];
        let mut active = BTreeSet::new();
        let mut versions = BTreeMap::new();
        let mut loaded = BTreeMap::new();
        let mut charged = 0u64;
        while let Some((identity, fingerprint, complete)) = pending.pop() {
            if complete {
                active.remove(&identity);
                continue;
            }
            if active.contains(&identity) {
                return Err(StoreError::Cycle(identity));
            }
            if let Some(previous) = versions.insert(identity.clone(), fingerprint) {
                if previous != fingerprint {
                    return Err(StoreError::VersionConflict(identity));
                }
                continue;
            }
            if loaded.len() >= self.options.max_graph_artifacts {
                return Err(StoreError::ResourceLimit);
            }
            if current {
                let (catalog, _) =
                    catalog::read(&self.root, &identity.library, self.options.limits)?;
                if catalog.lookup(&identity).map(|entry| entry.fingerprint) != Some(fingerprint) {
                    return Err(StoreError::Stale(identity));
                }
            }
            let size = fs::metadata(self.artifact_path(fingerprint))
                .map_err(|error| {
                    if error.kind() == std::io::ErrorKind::NotFound {
                        StoreError::Missing(identity.clone())
                    } else {
                        StoreError::Io(error)
                    }
                })?
                .len();
            charged = charged
                .checked_add(size)
                .filter(|charged| *charged <= self.options.max_graph_bytes)
                .ok_or(StoreError::ResourceLimit)?;
            let artifact = self.read_exact(&identity, fingerprint)?;
            active.insert(identity.clone());
            pending.push((identity.clone(), fingerprint, true));
            for dependency in artifact.dependencies.iter().rev() {
                pending.push((
                    dependency.identity.normalized(),
                    dependency.fingerprint,
                    false,
                ));
            }
            loaded.insert(identity, artifact);
        }
        Ok(ArtifactGraph {
            root: fingerprint,
            artifacts: loaded.into_values().collect(),
        })
    }
}
