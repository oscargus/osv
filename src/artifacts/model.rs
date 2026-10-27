use serde::{Deserialize, Serialize};

use crate::source::{ByteSpan, SourceFile};

pub const SCHEMA_VERSION: u32 = 1;
pub const COMPILER_COMPATIBILITY: u32 = 1;
pub const NATIVE_ABI_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum ArtifactStage {
    Analysis,
    Elaboration,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum LanguageRevision {
    Vhdl2008,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct UnitId(pub u32);

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct TypeId(pub u32);

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct SymbolId(pub u32);

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ContentFingerprint(pub [u8; 32]);

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum Identifier {
    Basic(String),
    Extended(String),
}

impl Identifier {
    pub fn normalized(&self) -> Self {
        match self {
            Self::Basic(name) => Self::Basic(name.to_lowercase()),
            Self::Extended(name) => Self::Extended(name.clone()),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum UnitKind {
    Entity,
    Package,
    Configuration,
    Context,
    Architecture { name: Identifier },
    PackageBody,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UnitIdentity {
    pub library: Identifier,
    pub primary: Identifier,
    pub kind: UnitKind,
}

impl UnitIdentity {
    pub fn normalized(&self) -> Self {
        let mut identity = self.clone();
        identity.library = identity.library.normalized();
        identity.primary = identity.primary.normalized();
        if let UnitKind::Architecture { name } = &mut identity.kind {
            *name = name.normalized();
        }
        identity
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRecord {
    pub file: SourceFile,
    pub fingerprint: ContentFingerprint,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UnitRecord {
    pub id: UnitId,
    pub identity: UnitIdentity,
    pub parent: Option<UnitId>,
    pub span: Option<ByteSpan>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ScalarType {
    Boolean,
    Bit,
    Integer,
    Real,
    Time,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TypeDefinition {
    Scalar(ScalarType),
    Enumeration { literals: Vec<Identifier> },
    Subtype { base: TypeId },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TypeRecord {
    pub id: TypeId,
    pub name: Identifier,
    pub declaration: Option<SymbolId>,
    pub definition: TypeDefinition,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SymbolKind {
    Type,
    Constant,
    Signal,
    Variable,
    Subprogram,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SymbolRecord {
    pub id: SymbolId,
    pub unit: UnitId,
    pub name: Identifier,
    pub kind: SymbolKind,
    pub type_id: Option<TypeId>,
    pub span: Option<ByteSpan>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum DependencyRole {
    Declaration,
    Body,
    ElaboratedDesign,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyRecord {
    pub identity: UnitIdentity,
    pub stage: ArtifactStage,
    pub role: DependencyRole,
    pub fingerprint: ContentFingerprint,
    pub schema_version: u32,
    pub compiler_compatibility: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NativeRequirement {
    pub library: Identifier,
    pub package: Identifier,
    pub declarations: ContentFingerprint,
    pub abi_version: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AnalysisArtifact {
    pub unit: UnitId,
    pub declarations: Vec<SymbolId>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ElaborationArtifact {
    pub root: UnitId,
    pub architecture: UnitId,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ArtifactPayload {
    Analysis(AnalysisArtifact),
    Elaboration(ElaborationArtifact),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactEnvelope {
    pub schema_version: u32,
    pub compiler_compatibility: u32,
    pub compiler_version: String,
    pub language: LanguageRevision,
    pub identity: UnitIdentity,
    pub sources: Vec<SourceRecord>,
    pub units: Vec<UnitRecord>,
    pub types: Vec<TypeRecord>,
    pub symbols: Vec<SymbolRecord>,
    pub dependencies: Vec<DependencyRecord>,
    pub native_requirements: Vec<NativeRequirement>,
    pub payload: ArtifactPayload,
}

impl ArtifactEnvelope {
    pub fn stage(&self) -> ArtifactStage {
        match self.payload {
            ArtifactPayload::Analysis(_) => ArtifactStage::Analysis,
            ArtifactPayload::Elaboration(_) => ArtifactStage::Elaboration,
        }
    }
}
