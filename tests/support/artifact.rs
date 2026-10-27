use std::path::Path;

use osv::{
    artifacts::*,
    source::{SourceFile, SourceId},
};

pub fn sample() -> ArtifactEnvelope {
    let identity = UnitIdentity {
        library: Identifier::Basic("work".into()),
        primary: Identifier::Basic("synthetic".into()),
        kind: UnitKind::Entity,
    };
    ArtifactEnvelope {
        schema_version: SCHEMA_VERSION,
        compiler_compatibility: COMPILER_COMPATIBILITY,
        compiler_version: env!("CARGO_PKG_VERSION").into(),
        language: LanguageRevision::Vhdl2008,
        identity: identity.clone(),
        sources: vec![
            SourceRecord::embedded(SourceFile::new(
                SourceId(4),
                Path::new("synthetic.vhd"),
                b"entity synthetic is end entity;",
            ))
            .unwrap(),
        ],
        units: vec![UnitRecord {
            id: UnitId(7),
            identity,
            parent: None,
            span: None,
        }],
        types: vec![TypeRecord {
            id: TypeId(3),
            name: Identifier::Basic("boolean".into()),
            declaration: None,
            definition: TypeDefinition::Scalar(ScalarType::Boolean),
        }],
        symbols: vec![SymbolRecord {
            id: SymbolId(5),
            unit: UnitId(7),
            name: Identifier::Basic("flag".into()),
            kind: SymbolKind::Signal,
            type_id: Some(TypeId(3)),
            span: None,
        }],
        dependencies: Vec::new(),
        native_requirements: Vec::new(),
        payload: ArtifactPayload::Analysis(AnalysisArtifact {
            unit: UnitId(7),
            declarations: vec![SymbolId(5)],
        }),
    }
}
