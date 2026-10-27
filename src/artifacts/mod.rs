mod bounded;
mod codec;
mod identity;
mod model;
mod validate;

pub use codec::{
    ArtifactFormat, CodecError, DecodeLimits, ResourceLimit, decode, encode, read_from,
};
pub use model::{
    AnalysisArtifact, ArtifactEnvelope, ArtifactPayload, ArtifactStage, COMPILER_COMPATIBILITY,
    ContentFingerprint, DependencyRecord, DependencyRole, ElaborationArtifact, Identifier,
    LanguageRevision, NATIVE_ABI_VERSION, NativeRequirement, SCHEMA_VERSION, ScalarType,
    SourceRecord, SymbolId, SymbolKind, SymbolRecord, TypeDefinition, TypeId, TypeRecord, UnitId,
    UnitIdentity, UnitKind, UnitRecord,
};
pub use validate::{ArtifactError, TableKind};
