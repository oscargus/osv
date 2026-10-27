use std::collections::{BTreeMap, BTreeSet};

use super::{ArtifactEnvelope, ArtifactError, ScopeKind, TableKind, validate::name};

impl ArtifactEnvelope {
    pub(super) fn validate_scopes(&self) -> Result<(), ArtifactError> {
        let scopes = self
            .scopes
            .iter()
            .map(|scope| (scope.id, scope))
            .collect::<BTreeMap<_, _>>();
        let units = self
            .units
            .iter()
            .map(|unit| (unit.id, unit))
            .collect::<BTreeMap<_, _>>();
        let mut libraries = BTreeMap::new();
        let mut unit_scopes = BTreeMap::new();
        for scope in &self.scopes {
            match &scope.kind {
                ScopeKind::Library(library) => {
                    name(library)?;
                    if scope.parent.is_some()
                        || libraries.insert(library.normalized(), scope.id).is_some()
                    {
                        return Err(ArtifactError::InvalidScope(scope.id));
                    }
                }
                ScopeKind::Unit(unit) => {
                    if !units.contains_key(unit) {
                        return Err(ArtifactError::DanglingReference {
                            table: TableKind::Unit,
                            id: unit.0,
                        });
                    }
                    if unit_scopes.insert(*unit, scope.id).is_some() {
                        return Err(ArtifactError::InvalidScope(scope.id));
                    }
                }
                ScopeKind::Lexical(unit) => {
                    if !units.contains_key(unit) {
                        return Err(ArtifactError::DanglingReference {
                            table: TableKind::Unit,
                            id: unit.0,
                        });
                    }
                }
            }
        }
        for unit in &self.units {
            let scope = unit_scopes
                .get(&unit.id)
                .ok_or(ArtifactError::MissingUnitScope(unit.id))?;
            let expected = match unit.parent {
                Some(parent) => unit_scopes
                    .get(&parent)
                    .copied()
                    .ok_or(ArtifactError::MissingUnitScope(parent))?,
                None => libraries
                    .get(&unit.identity.library.normalized())
                    .copied()
                    .ok_or(ArtifactError::InvalidScope(*scope))?,
            };
            if scopes[scope].parent != Some(expected) {
                return Err(ArtifactError::InvalidScope(*scope));
            }
        }
        for scope in &self.scopes {
            if let ScopeKind::Lexical(unit) = scope.kind {
                let parent = scope
                    .parent
                    .and_then(|parent| scopes.get(&parent))
                    .ok_or(ArtifactError::InvalidScope(scope.id))?;
                if !matches!(parent.kind, ScopeKind::Unit(owner) | ScopeKind::Lexical(owner) if owner == unit)
                {
                    return Err(ArtifactError::InvalidScope(scope.id));
                }
            }
        }
        let mut checked = BTreeSet::new();
        for scope in &self.scopes {
            let mut path = BTreeSet::new();
            let mut next = Some(scope.id);
            while let Some(id) = next {
                if checked.contains(&id) {
                    break;
                }
                if !path.insert(id) {
                    return Err(ArtifactError::InvalidScope(id));
                }
                next = scopes[&id].parent;
            }
            checked.extend(path);
        }
        for symbol in &self.symbols {
            if !units.contains_key(&symbol.unit) {
                return Err(ArtifactError::DanglingReference {
                    table: TableKind::Unit,
                    id: symbol.unit.0,
                });
            }
            let scope = scopes
                .get(&symbol.scope)
                .ok_or(ArtifactError::DanglingReference {
                    table: TableKind::Scope,
                    id: symbol.scope.0,
                })?;
            if !matches!(scope.kind, ScopeKind::Unit(unit) | ScopeKind::Lexical(unit) if unit == symbol.unit)
            {
                return Err(ArtifactError::InvalidSymbol(symbol.id.0));
            }
        }
        Ok(())
    }
}
