use crate::artifacts::*;

use std::{collections::BTreeMap, error::Error, fmt};

use super::{
    IntrinsicName, IntrinsicRegistry, NativeBinding, NativeLiteral, NativeType, NativeTypeShape,
};

#[derive(Debug)]
pub enum PreludeError {
    Encoding(postcard::Error),
    Validation(ArtifactError),
    IdExhausted,
    ConflictingPrelude,
}

impl fmt::Display for PreludeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Encoding(error) => error.fmt(formatter),
            Self::Validation(error) => error.fmt(formatter),
            Self::IdExhausted => formatter.write_str("prelude installation exhausted artifact IDs"),
            Self::ConflictingPrelude => formatter.write_str(
                "existing standard declarations are incomplete or not the trusted prelude",
            ),
        }
    }
}
impl Error for PreludeError {}
impl From<postcard::Error> for PreludeError {
    fn from(error: postcard::Error) -> Self {
        Self::Encoding(error)
    }
}
impl From<ArtifactError> for PreludeError {
    fn from(error: ArtifactError) -> Self {
        Self::Validation(error)
    }
}

#[derive(Clone, Debug)]
pub struct PreludeIds {
    pub unit: UnitId,
    pub scope: ScopeId,
    pub types: BTreeMap<NativeType, TypeId>,
}

fn next(ids: impl Iterator<Item = u32>) -> Result<u32, PreludeError> {
    match ids.max() {
        None => Ok(0),
        Some(id) => id.checked_add(1).ok_or(PreludeError::IdExhausted),
    }
}

fn symbol(
    symbols: &mut Vec<SymbolRecord>,
    name: Identifier,
    kind: SymbolKind,
    type_id: TypeId,
    native: NativeBinding,
) -> Result<SymbolId, PreludeError> {
    let id = SymbolId(u32::try_from(symbols.len()).map_err(|_| PreludeError::IdExhausted)?);
    symbols.push(SymbolRecord {
        id,
        unit: UnitId(0),
        scope: ScopeId(1),
        name,
        kind,
        type_id: Some(type_id),
        span: None,
        native: Some(native),
    });
    Ok(id)
}

pub fn standard_prelude() -> Result<ArtifactEnvelope, PreludeError> {
    let registry = IntrinsicRegistry::standard();
    let identity = UnitIdentity {
        library: Identifier::Basic("std".into()),
        primary: Identifier::Basic("standard".into()),
        kind: UnitKind::Package,
    };
    let mut symbols = Vec::new();
    let mut types = Vec::new();
    let mut ids = BTreeMap::new();
    for profile in registry.types() {
        let id = TypeId(u32::try_from(types.len()).map_err(|_| PreludeError::IdExhausted)?);
        ids.insert(profile.id, id);
        let declaration = symbol(
            &mut symbols,
            profile.name.clone(),
            SymbolKind::Type,
            id,
            NativeBinding::Type(profile.id),
        )?;
        types.push(TypeRecord {
            id,
            name: profile.name.clone(),
            declaration: Some(declaration),
            definition: TypeDefinition::Native(profile.id),
        });
    }
    for profile in registry.types() {
        match &profile.shape {
            NativeTypeShape::Enumeration { literals } => {
                for (position, literal) in literals.iter().enumerate() {
                    let name = match literal {
                        NativeLiteral::Identifier(name) => name.clone(),
                        NativeLiteral::Character(character) => Identifier::Character(*character),
                    };
                    symbol(
                        &mut symbols,
                        name,
                        SymbolKind::EnumerationLiteral,
                        ids[&profile.id],
                        NativeBinding::Literal {
                            type_id: profile.id,
                            position: u32::try_from(position)
                                .map_err(|_| PreludeError::IdExhausted)?,
                        },
                    )?;
                }
            }
            NativeTypeShape::Physical { units, .. } => {
                for unit in units {
                    symbol(
                        &mut symbols,
                        unit.name.clone(),
                        SymbolKind::PhysicalUnit,
                        ids[&profile.id],
                        NativeBinding::PhysicalUnit {
                            primary_units: unit.primary_units,
                        },
                    )?;
                }
            }
            _ => {}
        }
    }
    for attribute in registry.attributes() {
        symbol(
            &mut symbols,
            attribute.name.clone(),
            SymbolKind::Attribute,
            ids[&attribute.type_id],
            NativeBinding::Attribute {
                type_id: attribute.type_id,
            },
        )?;
    }
    for definition in registry.definitions() {
        let name = match &definition.signature.name {
            IntrinsicName::Operator(operator) => Identifier::Operator(*operator),
            IntrinsicName::Function(name) => name.clone(),
            IntrinsicName::Statement(_) => continue,
        };
        let declaration = registry
            .declaration(definition.id)?
            .ok_or(PreludeError::ConflictingPrelude)?;
        let reference = registry
            .resolve(&declaration)?
            .ok_or(PreludeError::ConflictingPrelude)?;
        let result = definition
            .signature
            .result
            .ok_or(PreludeError::ConflictingPrelude)?;
        symbol(
            &mut symbols,
            name,
            SymbolKind::Subprogram,
            ids[&result],
            NativeBinding::Subprogram(reference),
        )?;
    }
    let declarations = symbols.iter().map(|symbol| symbol.id).collect();
    let artifact = ArtifactEnvelope {
        schema_version: SCHEMA_VERSION,
        compiler_compatibility: COMPILER_COMPATIBILITY,
        compiler_version: env!("CARGO_PKG_VERSION").into(),
        language: LanguageRevision::Vhdl2008,
        identity: identity.clone(),
        sources: Vec::new(),
        units: vec![UnitRecord {
            id: UnitId(0),
            identity,
            parent: None,
            span: None,
        }],
        scopes: vec![
            ScopeRecord {
                id: ScopeId(0),
                parent: None,
                kind: ScopeKind::Library(Identifier::Basic("std".into())),
            },
            ScopeRecord {
                id: ScopeId(1),
                parent: Some(ScopeId(0)),
                kind: ScopeKind::Unit(UnitId(0)),
            },
        ],
        types,
        symbols,
        dependencies: Vec::new(),
        native_requirements: vec![NativeRequirement {
            library: Identifier::Basic("std".into()),
            package: Identifier::Basic("standard".into()),
            declarations: registry.declaration_fingerprint()?,
            abi_version: NATIVE_ABI_VERSION,
        }],
        payload: ArtifactPayload::Analysis(AnalysisArtifact {
            unit: UnitId(0),
            declarations,
            objects: Vec::new(),
            pending_processes: Vec::new(),
        }),
    };
    artifact.validate(ArtifactStage::Analysis)?;
    Ok(artifact)
}

fn installed(artifact: &ArtifactEnvelope, unit: UnitId) -> Result<PreludeIds, PreludeError> {
    let scope = artifact
        .scopes
        .iter()
        .find_map(|scope| (scope.kind == ScopeKind::Unit(unit)).then_some(scope.id))
        .ok_or(PreludeError::ConflictingPrelude)?;
    let mut types = BTreeMap::new();
    for symbol in artifact.symbols.iter().filter(|symbol| symbol.unit == unit) {
        if let Some(NativeBinding::Type(native)) = symbol.native
            && types
                .insert(
                    native,
                    symbol.type_id.ok_or(PreludeError::ConflictingPrelude)?,
                )
                .is_some()
        {
            return Err(PreludeError::ConflictingPrelude);
        }
    }
    if types.len() != IntrinsicRegistry::standard().types().len() {
        return Err(PreludeError::ConflictingPrelude);
    }
    let expected = standard_prelude()?;
    for required in &expected.symbols {
        if !artifact
            .symbols
            .iter()
            .filter(|symbol| symbol.unit == unit)
            .any(|symbol| {
                symbol.kind == required.kind
                    && symbol.name.normalized() == required.name.normalized()
                    && match (&symbol.native, &required.native) {
                        (
                            Some(NativeBinding::Subprogram(left)),
                            Some(NativeBinding::Subprogram(right)),
                        ) => left.id == right.id,
                        (left, right) => left == right,
                    }
            })
        {
            return Err(PreludeError::ConflictingPrelude);
        }
    }
    Ok(PreludeIds { unit, scope, types })
}

pub fn install_standard(artifact: &mut ArtifactEnvelope) -> Result<PreludeIds, PreludeError> {
    artifact.validate(artifact.stage())?;
    let prelude = standard_prelude()?;
    if let Some(unit) = artifact
        .units
        .iter()
        .find(|unit| unit.identity == prelude.identity)
    {
        return installed(artifact, unit.id);
    }
    let mut next_artifact = artifact.clone();
    let unit = UnitId(next(next_artifact.units.iter().map(|unit| unit.id.0))?);
    let library_scope = if let Some(scope) = next_artifact
        .scopes
        .iter()
        .find(|scope| scope.kind == ScopeKind::Library(Identifier::Basic("std".into())))
    {
        scope.id
    } else {
        let id = ScopeId(next(next_artifact.scopes.iter().map(|scope| scope.id.0))?);
        next_artifact.scopes.push(ScopeRecord {
            id,
            parent: None,
            kind: ScopeKind::Library(Identifier::Basic("std".into())),
        });
        id
    };
    let scope = ScopeId(next(next_artifact.scopes.iter().map(|scope| scope.id.0))?);
    next_artifact.scopes.push(ScopeRecord {
        id: scope,
        parent: Some(library_scope),
        kind: ScopeKind::Unit(unit),
    });
    next_artifact.units.push(UnitRecord {
        id: unit,
        identity: prelude.identity,
        parent: None,
        span: None,
    });
    let mut type_ids = BTreeMap::new();
    let mut symbol_ids = BTreeMap::new();
    let type_start = next(next_artifact.types.iter().map(|record| record.id.0))?;
    let symbol_start = next(next_artifact.symbols.iter().map(|symbol| symbol.id.0))?;
    for record in &prelude.types {
        type_ids.insert(
            record.id,
            TypeId(
                type_start
                    .checked_add(record.id.0)
                    .ok_or(PreludeError::IdExhausted)?,
            ),
        );
    }
    for record in &prelude.symbols {
        symbol_ids.insert(
            record.id,
            SymbolId(
                symbol_start
                    .checked_add(record.id.0)
                    .ok_or(PreludeError::IdExhausted)?,
            ),
        );
    }
    for mut record in prelude.types {
        record.id = type_ids[&record.id];
        record.declaration = record.declaration.map(|id| symbol_ids[&id]);
        next_artifact.types.push(record);
    }
    for mut record in prelude.symbols {
        record.id = symbol_ids[&record.id];
        record.unit = unit;
        record.scope = scope;
        record.type_id = record.type_id.map(|id| type_ids[&id]);
        next_artifact.symbols.push(record);
    }
    for requirement in prelude.native_requirements {
        if let Some(previous) = next_artifact.native_requirements.iter().find(|previous| {
            previous.library == requirement.library && previous.package == requirement.package
        }) {
            if previous != &requirement {
                return Err(PreludeError::ConflictingPrelude);
            }
        } else {
            next_artifact.native_requirements.push(requirement);
        }
    }
    next_artifact.validate(next_artifact.stage())?;
    let ids = installed(&next_artifact, unit)?;
    *artifact = next_artifact;
    Ok(ids)
}
