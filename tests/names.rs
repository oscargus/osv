#[path = "support/artifact.rs"]
mod support;

use std::path::Path;

use osv::{
    analysis::{UnitRegistry, names::ScopeLookup},
    artifacts::*,
    diagnostics::DiagnosticCode,
    source::{SourceFile, SourceId},
    syntax,
};

#[test]
fn name_lookup_uses_nearest_scope_and_basic_identifier_case() {
    let mut artifact = support::sample();
    artifact.scopes.push(ScopeRecord {
        id: ScopeId(2),
        parent: Some(ScopeId(1)),
        kind: ScopeKind::Lexical(UnitId(7)),
    });
    let mut inner = artifact.symbols[0].clone();
    inner.id = SymbolId(6);
    inner.scope = ScopeId(2);
    inner.name = Identifier::Basic("FLAG".into());
    artifact.symbols.push(inner);
    let lookup = ScopeLookup::new(&artifact).unwrap();
    assert_eq!(
        lookup
            .lookup(ScopeId(2), &Identifier::Basic("flag".into()))
            .unwrap()[0]
            .id,
        SymbolId(6)
    );
    assert_eq!(
        lookup
            .lookup(ScopeId(1), &Identifier::Basic("FLAG".into()))
            .unwrap()[0]
            .id,
        SymbolId(5)
    );
    assert!(
        lookup
            .lookup(ScopeId(2), &Identifier::Extended("FLAG".into()))
            .unwrap()
            .is_empty()
    );
}

fn parsed(name: &str, bytes: &[u8]) -> syntax::ParsedSource {
    syntax::parse(&SourceFile::new(SourceId(4), Path::new(name), bytes)).unwrap()
}

#[test]
fn name_registry_registers_real_units_secondaries_and_lexical_shadowing() {
    let parsed = parsed("names.vhd", include_bytes!("fixtures/vhdl/names-valid.vhd"));
    let mut registry = UnitRegistry::new(Identifier::Basic("custom".into()));
    let units = registry.register(&parsed).unwrap();
    assert_eq!(units.len(), 4);
    let entity = registry
        .lookup_unit(
            &Identifier::Basic("work".into()),
            &Identifier::Basic("ROOT".into()),
            &UnitKind::Entity,
        )
        .unwrap();
    assert_eq!(entity.identity.library, Identifier::Basic("custom".into()));
    let architecture = registry
        .lookup_unit(
            &Identifier::Basic("work".into()),
            &Identifier::Basic("root".into()),
            &UnitKind::Architecture {
                name: Identifier::Basic("SIM".into()),
            },
        )
        .unwrap();
    assert_eq!(architecture.parent, Some(entity.id));
    assert!(
        registry
            .library_scope(&Identifier::Basic("std".into()))
            .is_some()
    );
    let flags = registry
        .declarations()
        .iter()
        .filter(|declaration| declaration.name == Identifier::Basic("flag".into()))
        .collect::<Vec<_>>();
    assert_eq!(flags.len(), 2);
    let inner = flags
        .iter()
        .find(|declaration| {
            registry.scopes().iter().any(|scope| {
                scope.id == declaration.scope && matches!(scope.kind, ScopeKind::Lexical(_))
            })
        })
        .unwrap();
    assert_eq!(
        registry
            .lookup(inner.scope, &Identifier::Basic("FLAG".into()))
            .unwrap()[0]
            .id,
        inner.id
    );
}

#[test]
fn name_registry_duplicates_fail_transactionally_with_related_locations() {
    let parsed = parsed(
        "duplicate.vhd",
        include_bytes!("fixtures/vhdl/names-duplicate.vhd"),
    );
    let mut registry = UnitRegistry::new(Identifier::Basic("work".into()));
    let errors = registry.register(&parsed).unwrap_err();
    assert_eq!(errors[0].code, DiagnosticCode::DuplicateDeclaration);
    assert_eq!(errors[0].related.len(), 1);
    assert!(registry.units().is_empty());
    assert!(registry.declarations().is_empty());
}

#[test]
fn name_registry_forward_and_wrong_secondary_prerequisites_are_explicit() {
    let mut registry = UnitRegistry::new(Identifier::Basic("work".into()));
    let forward = parsed("forward.vhd", b"architecture sim of absent is begin end;");
    assert_eq!(
        registry.register(&forward).unwrap_err()[0].code,
        DiagnosticCode::MissingUnit
    );
    let package = syntax::parse(&SourceFile::new(
        SourceId(5),
        Path::new("package.vhd"),
        b"package absent is end package;",
    ))
    .unwrap();
    registry.register(&package).unwrap();
    let error = registry.register(&forward).unwrap_err();
    assert_eq!(error[0].code, DiagnosticCode::WrongSecondaryParent);
    assert_eq!(error[0].related.len(), 1);
    assert_eq!(error[0].related[0].span.source, SourceId(5));
}

#[test]
fn name_registry_extended_names_do_not_fold_or_alias_basic_names() {
    let parsed = parsed(
        "extended.vhd",
        include_bytes!("fixtures/vhdl/names-extended.vhd"),
    );
    let mut registry = UnitRegistry::new(Identifier::Basic("work".into()));
    registry.register(&parsed).unwrap();
    assert_eq!(registry.units().len(), 3);
    for name in [
        Identifier::Basic("mixed".into()),
        Identifier::Extended("Mixed".into()),
        Identifier::Extended("mixed".into()),
    ] {
        assert!(
            registry
                .lookup_unit(&Identifier::Basic("work".into()), &name, &UnitKind::Entity)
                .is_some()
        );
    }
}

#[test]
fn name_lookup_identity_survives_binary_and_ron_artifact_reload() {
    let original = support::sample();
    for format in [ArtifactFormat::Binary, ArtifactFormat::Ron] {
        let loaded = decode(
            &encode(&original, format).unwrap(),
            None,
            DecodeLimits::default(),
        )
        .unwrap();
        let registry = UnitRegistry::from_artifact(&loaded).unwrap();
        let declaration = registry
            .lookup(ScopeId(1), &Identifier::Basic("FLAG".into()))
            .unwrap()[0];
        assert_eq!(declaration.id, SymbolId(5));
        assert_eq!(declaration.unit, UnitId(7));
        assert_eq!(declaration.span, None);
    }
}

#[test]
fn name_registry_does_not_claim_unimplemented_use_visibility() {
    let mut registry = UnitRegistry::new(Identifier::Basic("work".into()));
    let use_clause = parsed("visibility.vhd", b"use std.standard.all; entity e is end;");
    assert_eq!(
        registry.register(&use_clause).unwrap_err()[0].code,
        DiagnosticCode::UnsupportedFeature
    );
    let parsed = parsed("library.vhd", b"library unavailable; entity e is end;");
    assert_eq!(
        registry.register(&parsed).unwrap_err()[0].code,
        DiagnosticCode::MissingLibrary
    );
}

#[test]
fn name_registry_separate_files_require_primary_first_and_retain_source_ids() {
    let mut registry = UnitRegistry::new(Identifier::Basic("work".into()));
    let primary = parsed("primary.vhd", b"entity separated is end entity;");
    let secondary = syntax::parse(&SourceFile::new(
        SourceId(9),
        Path::new("secondary.vhd"),
        b"architecture sim of separated is begin end;",
    ))
    .unwrap();
    registry.register(&primary).unwrap();
    registry.register(&secondary).unwrap();
    assert_eq!(registry.units()[1].parent, Some(registry.units()[0].id));
    assert_eq!(registry.units()[0].span.unwrap().source, SourceId(4));
    assert_eq!(registry.units()[1].span.unwrap().source, SourceId(9));
}

#[test]
fn name_registry_primary_names_share_the_library_namespace() {
    let mut registry = UnitRegistry::new(Identifier::Basic("work".into()));
    let parsed = parsed(
        "homographs.vhd",
        b"entity same_name is end; package SAME_NAME is end package;",
    );
    let errors = registry.register(&parsed).unwrap_err();
    assert_eq!(errors[0].code, DiagnosticCode::DuplicateDeclaration);
    assert!(registry.units().is_empty());
}
