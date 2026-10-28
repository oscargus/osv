use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

use super::ScopeId;
use super::SymbolId;
use crate::source::{ByteSpan, SourceId};

use super::{
    ArtifactEnvelope, ArtifactPayload, ArtifactStage, COMPILER_COMPATIBILITY, ContentFingerprint,
    DependencyRole, Identifier, NATIVE_ABI_VERSION, SCHEMA_VERSION, SymbolKind, TypeDefinition,
    UnitId, UnitIdentity, UnitKind,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TableKind {
    Source,
    Unit,
    Type,
    Symbol,
    Scope,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArtifactError {
    SchemaVersion(u32),
    CompilerCompatibility(u32),
    WrongStage {
        expected: ArtifactStage,
        actual: ArtifactStage,
    },
    DuplicateId {
        table: TableKind,
        id: u32,
    },
    DanglingReference {
        table: TableKind,
        id: u32,
    },
    DuplicateIdentity(UnitIdentity),
    InvalidUnit(UnitId),
    InvalidScope(ScopeId),
    MissingUnitScope(UnitId),
    IdentityMismatch,
    InvalidName,
    InvalidType(u32),
    InvalidSymbol(u32),
    InvalidObject(SymbolId),
    InvalidProcess(ScopeId),
    InvalidInitialization {
        symbol: SymbolId,
        message: String,
    },
    InvalidSpan(ByteSpan),
    SourceFingerprint(SourceId),
    MissingSourceBytes(SourceId),
    InvalidDependency(UnitIdentity),
    MissingDependency(UnitIdentity),
    NativeRequirement,
    Encoding(String),
}

impl fmt::Display for ArtifactError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SchemaVersion(version) => {
                write!(formatter, "unsupported artifact schema {version}")
            }
            Self::CompilerCompatibility(version) => {
                write!(formatter, "incompatible compiler epoch {version}")
            }
            Self::WrongStage { expected, actual } => write!(
                formatter,
                "expected {expected:?} artifact, found {actual:?}"
            ),
            Self::DuplicateId { table, id } => write!(formatter, "duplicate {table:?} ID {id}"),
            Self::DanglingReference { table, id } => write!(formatter, "missing {table:?} ID {id}"),
            Self::DuplicateIdentity(identity) => {
                write!(formatter, "duplicate unit identity {identity:?}")
            }
            Self::InvalidUnit(id) => write!(
                formatter,
                "invalid unit or secondary parent for ID {}",
                id.0
            ),
            Self::IdentityMismatch => {
                formatter.write_str("artifact identity does not match its payload unit")
            }
            Self::InvalidName => formatter.write_str("artifact name must be nonempty Latin-1 text"),
            Self::InvalidType(id) => write!(
                formatter,
                "invalid type definition or declaration for ID {id}"
            ),
            Self::InvalidSymbol(id) => write!(formatter, "invalid symbol declaration for ID {id}"),
            Self::InvalidObject(id) => {
                write!(formatter, "invalid analyzed object declaration {}", id.0)
            }
            Self::InvalidProcess(id) => write!(formatter, "invalid pending process scope {}", id.0),
            Self::InvalidInitialization { symbol, message } => write!(
                formatter,
                "invalid initialization for object {}: {message}",
                symbol.0
            ),
            Self::InvalidSpan(span) => write!(
                formatter,
                "invalid source {} byte span {}..{}",
                span.source.0, span.start, span.end
            ),
            Self::SourceFingerprint(id) => write!(
                formatter,
                "source {} fingerprint differs from its bytes",
                id.0
            ),
            Self::MissingSourceBytes(id) => {
                write!(formatter, "source {} has no embedded bytes", id.0)
            }
            Self::InvalidDependency(identity) => write!(
                formatter,
                "duplicate or incompatible dependency {identity:?}"
            ),
            Self::MissingDependency(identity) => write!(
                formatter,
                "secondary unit lacks its primary dependency {identity:?}"
            ),
            Self::NativeRequirement => {
                formatter.write_str("duplicate or incompatible native-library requirement")
            }
            Self::InvalidScope(id) => {
                write!(formatter, "invalid lexical scope or parent for ID {}", id.0)
            }
            Self::MissingUnitScope(id) => {
                write!(formatter, "unit {} has no unique persisted scope", id.0)
            }
            Self::Encoding(message) => {
                write!(formatter, "cannot encode canonical artifact: {message}")
            }
        }
    }
}

impl Error for ArtifactError {}

pub(super) fn name(identifier: &Identifier) -> Result<(), ArtifactError> {
    let text = match identifier {
        Identifier::Basic(text) | Identifier::Extended(text) => text,
        Identifier::Character(_) | Identifier::Operator(_) => return Ok(()),
    };
    if text.is_empty() || text.chars().any(|character| u32::from(character) > 255) {
        return Err(ArtifactError::InvalidName);
    }
    Ok(())
}

fn identity(identity: &UnitIdentity) -> Result<(), ArtifactError> {
    if !matches!(
        identity.library,
        Identifier::Basic(_) | Identifier::Extended(_)
    ) || !matches!(
        identity.primary,
        Identifier::Basic(_) | Identifier::Extended(_)
    ) {
        return Err(ArtifactError::InvalidName);
    }
    name(&identity.library)?;
    name(&identity.primary)?;
    if let UnitKind::Architecture { name: identifier } = &identity.kind {
        name(identifier)?;
    }
    Ok(())
}

fn unique(ids: impl IntoIterator<Item = u32>, table: TableKind) -> Result<(), ArtifactError> {
    let mut seen = BTreeSet::new();
    for id in ids {
        if !seen.insert(id) {
            return Err(ArtifactError::DuplicateId { table, id });
        }
    }
    Ok(())
}

fn reference<Id: Ord, Record>(
    records: &BTreeMap<Id, Record>,
    id: Id,
    raw: u32,
    table: TableKind,
) -> Result<&Record, ArtifactError> {
    records
        .get(&id)
        .ok_or(ArtifactError::DanglingReference { table, id: raw })
}

impl ArtifactEnvelope {
    pub fn validate(&self, expected: ArtifactStage) -> Result<(), ArtifactError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(ArtifactError::SchemaVersion(self.schema_version));
        }
        if self.compiler_compatibility != COMPILER_COMPATIBILITY {
            return Err(ArtifactError::CompilerCompatibility(
                self.compiler_compatibility,
            ));
        }
        if self.stage() != expected {
            return Err(ArtifactError::WrongStage {
                expected,
                actual: self.stage(),
            });
        }
        identity(&self.identity)?;
        unique(
            self.sources.iter().map(|record| record.file.id.0),
            TableKind::Source,
        )?;
        unique(self.units.iter().map(|record| record.id.0), TableKind::Unit)?;
        unique(
            self.scopes.iter().map(|record| record.id.0),
            TableKind::Scope,
        )?;
        unique(self.types.iter().map(|record| record.id.0), TableKind::Type)?;
        unique(
            self.symbols.iter().map(|record| record.id.0),
            TableKind::Symbol,
        )?;
        let sources = self
            .sources
            .iter()
            .map(|record| (record.file.id, record))
            .collect::<BTreeMap<_, _>>();
        let units = self
            .units
            .iter()
            .map(|record| (record.id, record))
            .collect::<BTreeMap<_, _>>();
        let types = self
            .types
            .iter()
            .map(|record| (record.id, record))
            .collect::<BTreeMap<_, _>>();
        let symbols = self
            .symbols
            .iter()
            .map(|record| (record.id, record))
            .collect::<BTreeMap<_, _>>();
        let scopes = self
            .scopes
            .iter()
            .map(|record| (record.id, record))
            .collect::<BTreeMap<_, _>>();
        for scope in &self.scopes {
            if let Some(parent) = scope.parent {
                reference(&scopes, parent, parent.0, TableKind::Scope)?;
            }
        }
        let span = |location: Option<ByteSpan>| -> Result<(), ArtifactError> {
            if let Some(location) = location {
                let source = reference(
                    &sources,
                    location.source,
                    location.source.0,
                    TableKind::Source,
                )?;
                if location.start > location.end
                    || source
                        .file
                        .bytes
                        .as_ref()
                        .is_some_and(|bytes| location.checked_range(bytes.len()).is_none())
                {
                    return Err(ArtifactError::InvalidSpan(location));
                }
            }
            Ok(())
        };
        for source in &self.sources {
            if let Some(bytes) = &source.file.bytes
                && (u32::try_from(bytes.len()).is_err()
                    || ContentFingerprint::of_bytes(bytes) != source.fingerprint)
            {
                return Err(ArtifactError::SourceFingerprint(source.file.id));
            }
        }
        let mut identities = BTreeSet::new();
        for unit in &self.units {
            identity(&unit.identity)?;
            if !identities.insert(unit.identity.normalized()) {
                return Err(ArtifactError::DuplicateIdentity(unit.identity.normalized()));
            }
            span(unit.span)?;
            let expected_parent = match unit.identity.kind {
                UnitKind::Architecture { .. } => Some(UnitKind::Entity),
                UnitKind::PackageBody => Some(UnitKind::Package),
                _ => None,
            };
            match (expected_parent, unit.parent) {
                (Some(kind), Some(parent)) => {
                    let parent = reference(&units, parent, parent.0, TableKind::Unit)?;
                    if parent.identity.kind != kind
                        || parent.identity.library.normalized()
                            != unit.identity.library.normalized()
                        || parent.identity.primary.normalized()
                            != unit.identity.primary.normalized()
                    {
                        return Err(ArtifactError::InvalidUnit(unit.id));
                    }
                }
                (None, None) => {}
                _ => return Err(ArtifactError::InvalidUnit(unit.id)),
            }
        }
        self.validate_scopes()?;
        let mut checked_types = BTreeSet::new();
        for record in &self.types {
            name(&record.name)?;
            if let Some(declaration) = record.declaration {
                let symbol = reference(&symbols, declaration, declaration.0, TableKind::Symbol)?;
                if symbol.kind != SymbolKind::Type || symbol.type_id != Some(record.id) {
                    return Err(ArtifactError::InvalidType(record.id.0));
                }
            }
            match &record.definition {
                TypeDefinition::Enumeration { literals } => {
                    let mut seen = BTreeSet::new();
                    if literals.is_empty() {
                        return Err(ArtifactError::InvalidType(record.id.0));
                    }
                    for literal in literals {
                        name(literal)?;
                        if !seen.insert(literal.normalized()) {
                            return Err(ArtifactError::InvalidType(record.id.0));
                        }
                    }
                }
                TypeDefinition::Subtype { .. } => {
                    let mut seen = BTreeSet::new();
                    let mut next = record;
                    while let TypeDefinition::Subtype { base } = next.definition {
                        if checked_types.contains(&next.id) {
                            break;
                        }
                        if !seen.insert(next.id) {
                            return Err(ArtifactError::InvalidType(record.id.0));
                        }
                        next = reference(&types, base, base.0, TableKind::Type)?;
                    }
                    checked_types.extend(seen);
                }
                TypeDefinition::Scalar(_) | TypeDefinition::Native(_) => {}
            }
        }
        for symbol in &self.symbols {
            name(&symbol.name)?;
            if matches!(symbol.name, Identifier::Operator(_))
                && symbol.kind != SymbolKind::Subprogram
            {
                return Err(ArtifactError::InvalidSymbol(symbol.id.0));
            }
            if matches!(symbol.name, Identifier::Character(_))
                && symbol.kind != SymbolKind::EnumerationLiteral
            {
                return Err(ArtifactError::InvalidSymbol(symbol.id.0));
            }
            reference(&units, symbol.unit, symbol.unit.0, TableKind::Unit)?;
            reference(&scopes, symbol.scope, symbol.scope.0, TableKind::Scope)?;
            if let Some(type_id) = symbol.type_id {
                reference(&types, type_id, type_id.0, TableKind::Type)?;
            } else if matches!(
                symbol.kind,
                SymbolKind::Type
                    | SymbolKind::Constant
                    | SymbolKind::Signal
                    | SymbolKind::Variable
                    | SymbolKind::EnumerationLiteral
                    | SymbolKind::PhysicalUnit
                    | SymbolKind::Attribute
            ) {
                return Err(ArtifactError::InvalidSymbol(symbol.id.0));
            }
            span(symbol.span)?;
        }
        let mut dependencies = BTreeSet::new();
        for dependency in &self.dependencies {
            identity(&dependency.identity)?;
            let expected_stage = match dependency.role {
                DependencyRole::Declaration | DependencyRole::Body => ArtifactStage::Analysis,
                DependencyRole::ElaboratedDesign => ArtifactStage::Elaboration,
            };
            if dependency.stage != expected_stage
                || dependency.schema_version != SCHEMA_VERSION
                || dependency.compiler_compatibility != COMPILER_COMPATIBILITY
                || !dependencies.insert((
                    dependency.identity.normalized(),
                    dependency.role,
                    dependency.stage,
                ))
            {
                return Err(ArtifactError::InvalidDependency(
                    dependency.identity.clone(),
                ));
            }
        }
        let mut native = BTreeSet::new();
        for requirement in &self.native_requirements {
            name(&requirement.library)?;
            name(&requirement.package)?;
            if requirement.abi_version != NATIVE_ABI_VERSION
                || !native.insert((
                    requirement.library.normalized(),
                    requirement.package.normalized(),
                ))
            {
                return Err(ArtifactError::NativeRequirement);
            }
        }
        self.validate_native()?;
        self.validate_unit_data()?;
        let root = match &self.payload {
            ArtifactPayload::Analysis(analysis) => {
                let unit = reference(&units, analysis.unit, analysis.unit.0, TableKind::Unit)?;
                let mut declarations = BTreeSet::new();
                for declaration in &analysis.declarations {
                    let symbol =
                        reference(&symbols, *declaration, declaration.0, TableKind::Symbol)?;
                    if symbol.unit != unit.id || !declarations.insert(*declaration) {
                        return Err(ArtifactError::InvalidSymbol(declaration.0));
                    }
                }
                if let Some(parent) = unit.parent {
                    let parent = units[&parent];
                    if !self.dependencies.iter().any(|dependency| {
                        dependency.identity.normalized() == parent.identity.normalized()
                            && dependency.role == DependencyRole::Declaration
                    }) {
                        return Err(ArtifactError::MissingDependency(parent.identity.clone()));
                    }
                }
                unit
            }
            ArtifactPayload::Elaboration(elaboration) => {
                let root = reference(
                    &units,
                    elaboration.root,
                    elaboration.root.0,
                    TableKind::Unit,
                )?;
                let architecture = reference(
                    &units,
                    elaboration.architecture,
                    elaboration.architecture.0,
                    TableKind::Unit,
                )?;
                if root.identity.kind != UnitKind::Entity
                    || !matches!(architecture.identity.kind, UnitKind::Architecture { .. })
                    || architecture.parent != Some(root.id)
                {
                    return Err(ArtifactError::InvalidUnit(elaboration.architecture));
                }
                root
            }
        };
        if root.identity.normalized() != self.identity.normalized() {
            return Err(ArtifactError::IdentityMismatch);
        }
        Ok(())
    }
}
