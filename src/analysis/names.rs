use crate::artifacts::{
    ArtifactEnvelope, ArtifactError, Identifier, ScopeId, SymbolRecord, TableKind,
};

pub struct ScopeLookup<'artifact> {
    artifact: &'artifact ArtifactEnvelope,
}

impl<'artifact> ScopeLookup<'artifact> {
    pub fn new(artifact: &'artifact ArtifactEnvelope) -> Result<Self, ArtifactError> {
        artifact.validate(artifact.stage())?;
        Ok(Self { artifact })
    }

    pub fn lookup(
        &self,
        mut scope: ScopeId,
        name: &Identifier,
    ) -> Result<Vec<&'artifact SymbolRecord>, ArtifactError> {
        let name = name.normalized();
        loop {
            let record = self
                .artifact
                .scopes
                .iter()
                .find(|record| record.id == scope)
                .ok_or(ArtifactError::DanglingReference {
                    table: TableKind::Scope,
                    id: scope.0,
                })?;
            let declarations = self
                .artifact
                .symbols
                .iter()
                .filter(|symbol| symbol.scope == scope && symbol.name.normalized() == name)
                .collect::<Vec<_>>();
            if !declarations.is_empty() {
                return Ok(declarations);
            }
            let Some(parent) = record.parent else {
                return Ok(Vec::new());
            };
            scope = parent;
        }
    }
}
