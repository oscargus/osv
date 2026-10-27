#[path = "support/artifact.rs"]
mod support;

use osv::{analysis::names::ScopeLookup, artifacts::*, intrinsics::*};

#[test]
fn native_prelude_resolves_boolean_literals_without_sources_or_body() {
    let prelude = standard_prelude().unwrap();
    assert!(prelude.sources.is_empty());
    assert_eq!(prelude.units.len(), 1);
    assert_eq!(prelude.units[0].identity.kind, UnitKind::Package);
    let lookup = ScopeLookup::new(&prelude).unwrap();
    for name in ["boolean", "false", "true"] {
        assert_eq!(
            lookup
                .lookup(ScopeId(1), &Identifier::Basic(name.into()))
                .unwrap()
                .len(),
            1
        );
    }
}

#[test]
fn native_prelude_installs_audited_types_units_literals_and_operators() {
    let prelude = standard_prelude().unwrap();
    let lookup = ScopeLookup::new(&prelude).unwrap();
    for name in [
        "integer",
        "real",
        "time",
        "severity_level",
        "character",
        "string",
        "natural",
        "positive",
        "delay_length",
        "ns",
        "note",
        "failure",
        "now",
        "foreign",
    ] {
        assert_eq!(
            lookup
                .lookup(ScopeId(1), &Identifier::Basic(name.into()))
                .unwrap()
                .len(),
            1,
            "{name}"
        );
    }
    assert_eq!(prelude.types.len(), 18);
    assert!(
        lookup
            .lookup(ScopeId(1), &Identifier::Operator(Operator::Add))
            .unwrap()
            .len()
            > 1
    );
    assert_eq!(
        lookup
            .lookup(ScopeId(1), &Identifier::Character(b'0'))
            .unwrap()
            .len(),
        2
    );
    assert!(prelude.symbols.iter().all(|symbol| symbol.native.is_some()));
}

#[test]
fn native_prelude_installation_preserves_ids_and_implicit_lookup_after_both_codecs() {
    let mut artifact = support::sample();
    let ids = install_standard(&mut artifact).unwrap();
    assert_eq!(artifact.symbols[0].id, SymbolId(5));
    assert_eq!(artifact.units[0].id, UnitId(7));
    assert_eq!(artifact.symbols[0].type_id, Some(TypeId(3)));
    let count = artifact.symbols.len();
    assert_eq!(install_standard(&mut artifact).unwrap().unit, ids.unit);
    assert_eq!(artifact.symbols.len(), count);
    for format in [ArtifactFormat::Binary, ArtifactFormat::Ron] {
        let loaded = decode(
            &encode(&artifact, format).unwrap(),
            None,
            DecodeLimits::default(),
        )
        .unwrap();
        let lookup = ScopeLookup::new(&loaded).unwrap();
        let integer = lookup
            .lookup_with_standard(ScopeId(1), &Identifier::Basic("INTEGER".into()))
            .unwrap()[0];
        assert_eq!(
            integer.native,
            Some(NativeBinding::Type(NativeType::Integer))
        );
        assert_eq!(integer.type_id, Some(ids.types[&NativeType::Integer]));
    }
}

#[test]
fn native_prelude_cannot_replace_user_homonyms_or_accept_forged_bindings() {
    let mut artifact = support::sample();
    let mut user = artifact.symbols[0].clone();
    user.id = SymbolId(6);
    user.name = Identifier::Basic("now".into());
    user.kind = SymbolKind::Subprogram;
    artifact.symbols.push(user);
    install_standard(&mut artifact).unwrap();
    let lookup = ScopeLookup::new(&artifact).unwrap();
    let now = lookup
        .lookup_with_standard(ScopeId(1), &Identifier::Basic("now".into()))
        .unwrap()[0];
    assert_eq!(now.id, SymbolId(6));
    assert_eq!(now.native, None);
    let registry = IntrinsicRegistry::standard();
    let reference = registry
        .resolve(&registry.declaration(IntrinsicId::Now).unwrap().unwrap())
        .unwrap()
        .unwrap();
    artifact.symbols[1].native = Some(NativeBinding::Subprogram(reference));
    assert!(artifact.validate(artifact.stage()).is_err());
}

#[test]
fn native_prelude_duplicate_incomplete_and_stale_bindings_fail_explicitly() {
    let mut artifact = support::sample();
    install_standard(&mut artifact).unwrap();
    let mut duplicate = artifact
        .symbols
        .iter()
        .find(|symbol| {
            matches!(
                symbol.native,
                Some(NativeBinding::Type(NativeType::Integer))
            )
        })
        .unwrap()
        .clone();
    duplicate.id = SymbolId(1000);
    artifact.symbols.push(duplicate);
    assert!(artifact.validate(artifact.stage()).is_err());
    artifact.symbols.pop();
    artifact.symbols.retain(|symbol| !matches!(&symbol.native, Some(NativeBinding::Subprogram(reference)) if reference.id == IntrinsicId::Now));
    artifact.validate(artifact.stage()).unwrap();
    assert!(matches!(
        install_standard(&mut artifact),
        Err(PreludeError::ConflictingPrelude)
    ));
    let mut stale = standard_prelude().unwrap();
    stale.native_requirements[0].declarations =
        ContentFingerprint::of_bytes(b"not this registry version");
    assert!(stale.validate(stale.stage()).is_err());
}

#[test]
fn native_prelude_id_exhaustion_is_transactional_and_old_schemas_are_rejected() {
    let mut artifact = support::sample();
    artifact.scopes.push(ScopeRecord {
        id: ScopeId(u32::MAX),
        parent: Some(ScopeId(1)),
        kind: ScopeKind::Lexical(UnitId(7)),
    });
    let original = artifact.clone();
    assert!(matches!(
        install_standard(&mut artifact),
        Err(PreludeError::IdExhausted)
    ));
    assert_eq!(artifact, original);
    for schema_version in [1, 2] {
        let mut previous = support::sample();
        previous.schema_version = schema_version;
        assert!(
            matches!(encode(&previous, ArtifactFormat::Binary), Err(CodecError::Validation(ArtifactError::SchemaVersion(actual))) if actual == schema_version)
        );
    }
}
