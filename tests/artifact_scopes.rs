#[path = "support/artifact.rs"]
mod support;

use osv::artifacts::*;

#[test]
fn artifact_symbols_cannot_reference_missing_persisted_scopes() {
    let mut artifact = support::sample();
    artifact.validate(ArtifactStage::Analysis).unwrap();
    artifact.symbols[0].scope = ScopeId(99);
    assert_eq!(
        artifact.validate(ArtifactStage::Analysis),
        Err(ArtifactError::DanglingReference {
            table: TableKind::Scope,
            id: 99
        })
    );
}

#[test]
fn artifact_scope_parent_cycles_duplicate_ids_and_ownership_are_rejected() {
    let mut artifact = support::sample();
    artifact.scopes.push(artifact.scopes[0].clone());
    assert!(matches!(
        artifact.validate(artifact.stage()),
        Err(ArtifactError::DuplicateId {
            table: TableKind::Scope,
            ..
        })
    ));
    artifact.scopes.pop();
    artifact.scopes.extend([
        ScopeRecord {
            id: ScopeId(2),
            parent: Some(ScopeId(3)),
            kind: ScopeKind::Lexical(UnitId(7)),
        },
        ScopeRecord {
            id: ScopeId(3),
            parent: Some(ScopeId(2)),
            kind: ScopeKind::Lexical(UnitId(7)),
        },
    ]);
    assert!(matches!(
        artifact.validate(artifact.stage()),
        Err(ArtifactError::InvalidScope(_))
    ));
    artifact.scopes.truncate(2);
    artifact.symbols[0].scope = ScopeId(0);
    assert_eq!(
        artifact.validate(artifact.stage()),
        Err(ArtifactError::InvalidSymbol(5))
    );
    artifact.symbols[0].scope = ScopeId(1);
    artifact.scopes[1].parent = Some(ScopeId(99));
    assert!(matches!(
        artifact.validate(artifact.stage()),
        Err(ArtifactError::DanglingReference {
            table: TableKind::Scope,
            id: 99
        })
    ));
}

#[test]
fn artifact_scopes_preserve_symbol_identity_and_canonical_parity() {
    let mut artifact = support::sample();
    artifact.scopes.push(ScopeRecord {
        id: ScopeId(4),
        parent: Some(ScopeId(1)),
        kind: ScopeKind::Lexical(UnitId(7)),
    });
    artifact.symbols[0].scope = ScopeId(4);
    let fingerprint = artifact.fingerprint().unwrap();
    for format in [ArtifactFormat::Binary, ArtifactFormat::Ron] {
        let bytes = encode(&artifact, format).unwrap();
        let loaded = decode(
            &bytes,
            Some(ArtifactStage::Analysis),
            DecodeLimits::default(),
        )
        .unwrap();
        assert_eq!(loaded.symbols[0].id, SymbolId(5));
        assert_eq!(loaded.symbols[0].scope, ScopeId(4));
        assert_eq!(loaded.fingerprint().unwrap(), fingerprint);
    }
    artifact.scopes.reverse();
    assert_eq!(artifact.fingerprint().unwrap(), fingerprint);
    artifact.schema_version = 1;
    let old = ron::ser::to_string_pretty(&artifact, ron::ser::PrettyConfig::default()).unwrap();
    assert!(matches!(
        decode(old.as_bytes(), None, DecodeLimits::default()),
        Err(CodecError::Validation(ArtifactError::SchemaVersion(1)))
    ));
}

#[test]
fn artifact_scopes_require_unique_unit_library_and_lexical_ownership() {
    let original = support::sample();
    let mut missing = original.clone();
    missing.scopes.pop();
    assert_eq!(
        missing.validate(missing.stage()),
        Err(ArtifactError::MissingUnitScope(UnitId(7)))
    );
    let mut duplicate = original.clone();
    duplicate.scopes.push(ScopeRecord {
        id: ScopeId(2),
        parent: Some(ScopeId(0)),
        kind: ScopeKind::Unit(UnitId(7)),
    });
    assert!(matches!(
        duplicate.validate(duplicate.stage()),
        Err(ArtifactError::InvalidScope(_))
    ));
    let mut wrong_library = original.clone();
    wrong_library.scopes[0].kind = ScopeKind::Library(Identifier::Basic("other".into()));
    assert!(matches!(
        wrong_library.validate(wrong_library.stage()),
        Err(ArtifactError::InvalidScope(_))
    ));
    let mut unknown_owner = original;
    unknown_owner.scopes.push(ScopeRecord {
        id: ScopeId(2),
        parent: Some(ScopeId(1)),
        kind: ScopeKind::Lexical(UnitId(99)),
    });
    assert_eq!(
        unknown_owner.validate(unknown_owner.stage()),
        Err(ArtifactError::DanglingReference {
            table: TableKind::Unit,
            id: 99
        })
    );
}

#[test]
fn artifact_scope_identity_normalizes_library_case_but_preserves_sparse_ids() {
    let mut artifact = support::sample();
    let fingerprint = artifact.fingerprint().unwrap();
    artifact.scopes[0].kind = ScopeKind::Library(Identifier::Basic("WORK".into()));
    assert_eq!(artifact.fingerprint().unwrap(), fingerprint);
    artifact.scopes.push(ScopeRecord {
        id: ScopeId(u32::MAX),
        parent: Some(ScopeId(1)),
        kind: ScopeKind::Lexical(UnitId(7)),
    });
    artifact.symbols[0].scope = ScopeId(u32::MAX);
    let bytes = encode(&artifact, ArtifactFormat::Binary).unwrap();
    let loaded = decode(&bytes, None, DecodeLimits::default()).unwrap();
    assert_eq!(loaded.symbols[0].scope, ScopeId(u32::MAX));
}
