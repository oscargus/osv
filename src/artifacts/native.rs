use crate::intrinsics::{
    IntrinsicName, IntrinsicRegistry, NativeBinding, NativeLiteral, NativeType, NativeTypeShape,
};
use std::collections::BTreeSet;

use super::{
    ArtifactEnvelope, ArtifactError, Identifier, NATIVE_ABI_VERSION, SymbolKind, SymbolRecord,
    TypeDefinition, UnitKind,
};

fn literal_name(literal: &NativeLiteral) -> Identifier {
    match literal {
        NativeLiteral::Identifier(name) => name.clone(),
        NativeLiteral::Character(character) => Identifier::Character(*character),
    }
}

impl ArtifactEnvelope {
    fn native_type(&self, symbol: &SymbolRecord, expected: NativeType) -> bool {
        symbol
            .type_id
            .and_then(|id| self.types.iter().find(|record| record.id == id))
            .is_some_and(|record| record.definition == TypeDefinition::Native(expected))
    }

    pub(super) fn validate_native(&self) -> Result<(), ArtifactError> {
        if self.symbols.iter().all(|symbol| symbol.native.is_none())
            && self
                .types
                .iter()
                .all(|record| !matches!(record.definition, TypeDefinition::Native(_)))
        {
            return Ok(());
        }
        let registry = IntrinsicRegistry::standard();
        let fingerprint = registry
            .declaration_fingerprint()
            .map_err(|error| ArtifactError::Encoding(error.to_string()))?;
        for record in &self.types {
            if let TypeDefinition::Native(native) = record.definition {
                if record.name.normalized() != Identifier::Basic(native.name().into()) {
                    return Err(ArtifactError::InvalidType(record.id.0));
                }
                let declaration = record
                    .declaration
                    .and_then(|id| self.symbols.iter().find(|symbol| symbol.id == id));
                if declaration
                    .is_none_or(|symbol| symbol.native != Some(NativeBinding::Type(native)))
                {
                    return Err(ArtifactError::InvalidType(record.id.0));
                }
            }
        }
        let mut bindings = BTreeSet::new();
        for symbol in &self.symbols {
            let Some(binding) = &symbol.native else {
                continue;
            };
            let mut canonical_binding = binding.clone();
            if let NativeBinding::Subprogram(reference) = &mut canonical_binding {
                reference.declaration.unit = reference.declaration.unit.normalized();
                if let IntrinsicName::Function(name) = &mut reference.declaration.signature.name {
                    *name = name.normalized();
                }
            }
            let key = postcard::to_allocvec(&canonical_binding)
                .map_err(|error| ArtifactError::Encoding(error.to_string()))?;
            if !bindings.insert(key) {
                return Err(ArtifactError::InvalidSymbol(symbol.id.0));
            }
            let unit = self
                .units
                .iter()
                .find(|unit| unit.id == symbol.unit)
                .ok_or(ArtifactError::InvalidSymbol(symbol.id.0))?;
            if !self.scopes.iter().any(|scope| {
                scope.id == symbol.scope && scope.kind == super::ScopeKind::Unit(unit.id)
            }) {
                return Err(ArtifactError::InvalidSymbol(symbol.id.0));
            }
            let trusted = unit.identity.library.normalized() == Identifier::Basic("std".into())
                && unit.identity.primary.normalized() == Identifier::Basic("standard".into())
                && unit.identity.kind == UnitKind::Package;
            if !trusted
                || !self.native_requirements.iter().any(|required| {
                    required.library.normalized() == Identifier::Basic("std".into())
                        && required.package.normalized() == Identifier::Basic("standard".into())
                        && required.declarations == fingerprint
                        && required.abi_version == NATIVE_ABI_VERSION
                })
            {
                return Err(ArtifactError::NativeRequirement);
            }
            let valid = match binding {
                NativeBinding::Type(native) => {
                    symbol.kind == SymbolKind::Type
                        && symbol.name.normalized() == Identifier::Basic(native.name().into())
                        && self.native_type(symbol, *native)
                }
                NativeBinding::Literal { type_id, position } => {
                    let profile = registry
                        .types()
                        .iter()
                        .find(|profile| profile.id == *type_id);
                    let literal = profile.and_then(|profile| match &profile.shape {
                        NativeTypeShape::Enumeration { literals } => {
                            literals.get(usize::try_from(*position).ok()?)
                        }
                        _ => None,
                    });
                    symbol.kind == SymbolKind::EnumerationLiteral
                        && self.native_type(symbol, *type_id)
                        && literal.is_some_and(|literal| {
                            literal_name(literal).normalized() == symbol.name.normalized()
                        })
                }
                NativeBinding::PhysicalUnit { primary_units } => {
                    let profile = registry
                        .types()
                        .iter()
                        .find(|profile| profile.id == NativeType::Time);
                    symbol.kind == SymbolKind::PhysicalUnit
                        && self.native_type(symbol, NativeType::Time)
                        && profile.is_some_and(|profile| match &profile.shape {
                            NativeTypeShape::Physical { units, .. } => units.iter().any(|unit| {
                                unit.primary_units == *primary_units
                                    && unit.name.normalized() == symbol.name.normalized()
                            }),
                            _ => false,
                        })
                }
                NativeBinding::Attribute { type_id } => {
                    symbol.kind == SymbolKind::Attribute
                        && self.native_type(symbol, *type_id)
                        && registry.attributes().iter().any(|attribute| {
                            attribute.type_id == *type_id
                                && attribute.name.normalized() == symbol.name.normalized()
                        })
                }
                NativeBinding::Subprogram(reference) => {
                    let definition = registry
                        .definition(reference)
                        .map_err(|error| ArtifactError::Encoding(error.to_string()))?;
                    symbol.kind == SymbolKind::Subprogram
                        && definition.is_some_and(|definition| {
                            let name_matches = match &definition.signature.name {
                                IntrinsicName::Operator(operator) => {
                                    symbol.name == Identifier::Operator(*operator)
                                }
                                IntrinsicName::Function(name) => {
                                    symbol.name.normalized() == name.normalized()
                                }
                                IntrinsicName::Statement(_) => false,
                            };
                            name_matches
                                && match definition.signature.result {
                                    Some(result) => self.native_type(symbol, result),
                                    None => symbol.type_id.is_none(),
                                }
                        })
                }
            };
            if !valid {
                return Err(ArtifactError::InvalidSymbol(symbol.id.0));
            }
        }
        Ok(())
    }
}
