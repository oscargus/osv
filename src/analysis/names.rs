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

    pub fn lookup_with_standard(
        &self,
        scope: ScopeId,
        name: &Identifier,
    ) -> Result<Vec<&'artifact SymbolRecord>, ArtifactError> {
        let found = self.lookup(scope, name)?;
        if !found.is_empty() {
            return Ok(found);
        }
        let unit = self.artifact.units.iter().find(|unit| {
            unit.identity.library.normalized() == Identifier::Basic("std".into())
                && unit.identity.primary.normalized() == Identifier::Basic("standard".into())
                && unit.identity.kind == crate::artifacts::UnitKind::Package
        });
        let Some(unit) = unit else {
            return Ok(Vec::new());
        };
        let scope = self
            .artifact
            .scopes
            .iter()
            .find(|scope| scope.kind == crate::artifacts::ScopeKind::Unit(unit.id))
            .ok_or(ArtifactError::MissingUnitScope(unit.id))?;
        self.lookup(scope.id, name)
    }
}
