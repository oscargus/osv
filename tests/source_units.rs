use std::path::Path;

use std::collections::BTreeMap;

use osv::{
    analysis::{ObjectInitialization, analyze_units, constant::FoldResult},
    artifacts::{
        store::{LibraryStore, StoreError, StoreOptions},
        *,
    },
    diagnostics::DiagnosticCode,
    source::SourceMap,
    values::*,
};

#[test]
fn unit_analysis_empty_entity_returns_owned_typed_artifact() {
    let mut sources = SourceMap::default();
    sources
        .add(
            Path::new("empty.vhd"),
            include_bytes!("fixtures/vhdl/unit-empty.vhd"),
        )
        .unwrap();
    let artifacts = analyze_units(&sources, Identifier::Basic("work".into())).unwrap();
    assert_eq!(artifacts.len(), 1);
    let artifact = &artifacts[0];
    assert_eq!(artifact.identity.kind, UnitKind::Entity);
    assert_eq!(
        artifact.identity.primary,
        Identifier::Basic("empty_unit".into())
    );
    assert!(
        artifact
            .symbols
            .iter()
            .any(|symbol| symbol.name == Identifier::Basic("boolean".into()))
    );
    for format in [ArtifactFormat::Binary, ArtifactFormat::Ron] {
        let loaded = decode(
            &encode(artifact, format).unwrap(),
            Some(ArtifactStage::Analysis),
            DecodeLimits::default(),
        )
        .unwrap();
        assert_eq!(loaded, artifact.canonicalized().unwrap());
    }
}

#[test]
fn unit_analysis_real_entity_architecture_preserves_defaults_dependencies_and_process_boundaries() {
    let mut sources = SourceMap::default();
    sources
        .add(
            Path::new("units.vhd"),
            include_bytes!("fixtures/vhdl/unit-design.vhd"),
        )
        .unwrap();
    let artifacts = analyze_units(&sources, Identifier::Basic("work".into())).unwrap();
    assert_eq!(artifacts.len(), 2);
    let entity = &artifacts[0];
    let architecture = &artifacts[1];
    assert_eq!(entity.identity.kind, UnitKind::Entity);
    assert!(matches!(
        architecture.identity.kind,
        UnitKind::Architecture { .. }
    ));
    assert_eq!(
        architecture.dependencies[0].fingerprint,
        entity.fingerprint().unwrap()
    );
    let ArtifactPayload::Analysis(analysis) = &architecture.payload else {
        unreachable!()
    };
    assert_eq!(analysis.objects.len(), 3);
    assert_eq!(analysis.pending_processes.len(), 1);
    assert_eq!(
        analysis.pending_processes[0].name,
        Some(Identifier::Basic("worker".into()))
    );
    let count = architecture
        .symbols
        .iter()
        .find(|symbol| symbol.name == Identifier::Basic("count".into()))
        .unwrap();
    let initialization = &analysis
        .objects
        .iter()
        .find(|object| object.symbol == count.id)
        .unwrap()
        .initialization;
    assert_eq!(
        initialization,
        &ObjectInitialization::Default(PrimitiveValue::Integer(IntegerValue(i64::MIN)))
    );
    let done = architecture
        .symbols
        .iter()
        .find(|symbol| symbol.name == Identifier::Basic("done".into()))
        .unwrap();
    let ObjectInitialization::Expression(plan) = &analysis
        .objects
        .iter()
        .find(|object| object.symbol == done.id)
        .unwrap()
        .initialization
    else {
        unreachable!()
    };
    assert_eq!(
        plan.evaluate(architecture, &BTreeMap::new()).unwrap(),
        FoldResult::Value(PrimitiveValue::Boolean(false))
    );
    for artifact in &artifacts {
        for format in [ArtifactFormat::Binary, ArtifactFormat::Ron] {
            let loaded = decode(
                &encode(artifact, format).unwrap(),
                None,
                DecodeLimits::default(),
            )
            .unwrap();
            assert_eq!(loaded, artifact.canonicalized().unwrap());
        }
    }
    let directory = tempfile::tempdir().unwrap();
    let mut store = LibraryStore::open(
        directory.path(),
        Identifier::Basic("work".into()),
        StoreOptions {
            native_requirements: entity.native_requirements.clone(),
            ..StoreOptions::default()
        },
    )
    .unwrap();
    store.publish(entity, ArtifactFormat::Binary).unwrap();
    assert!(matches!(
        store.publish(architecture, ArtifactFormat::Ron),
        Err(StoreError::IncompleteAnalysis(_))
    ));
}

fn diagnostics(bytes: &[u8]) -> Vec<osv::diagnostics::Diagnostic> {
    let mut sources = SourceMap::default();
    sources.add(Path::new("invalid-unit.vhd"), bytes).unwrap();
    analyze_units(&sources, Identifier::Basic("work".into())).unwrap_err()
}

#[test]
fn unit_analysis_rejects_missing_wrong_and_duplicate_declarations() {
    assert_eq!(
        diagnostics(b"architecture sim of absent is begin end;")[0].code,
        DiagnosticCode::MissingUnit
    );
    assert_eq!(
        diagnostics(b"package p is end; architecture sim of p is begin end;")[0].code,
        DiagnosticCode::WrongSecondaryParent
    );
    let errors = diagnostics(include_bytes!("fixtures/vhdl/names-duplicate.vhd"));
    assert_eq!(errors[0].code, DiagnosticCode::DuplicateDeclaration);
    assert_eq!(errors[0].related.len(), 1);
    assert_eq!(
        diagnostics(b"entity e is end wrong;")[0].code,
        DiagnosticCode::NameMismatch
    );
}

#[test]
fn unit_analysis_rejects_initializer_type_range_and_missing_value_errors() {
    for (bytes, code) in [
        (
            b"entity e is end; architecture sim of e is signal flag : boolean := 42; begin end;"
                .as_slice(),
            DiagnosticCode::TypeMismatch,
        ),
        (
            b"entity e is end; architecture sim of e is constant value : integer; begin end;"
                .as_slice(),
            DiagnosticCode::MissingInitializer,
        ),
        (
            b"entity e is end; architecture sim of e is signal value : unknown_type; begin end;"
                .as_slice(),
            DiagnosticCode::MissingName,
        ),
        (
            b"entity e is end; architecture sim of e is constant value : positive := 0; begin end;"
                .as_slice(),
            DiagnosticCode::Arithmetic,
        ),
    ] {
        let errors = diagnostics(bytes);
        assert_eq!(errors[0].code, code, "{errors:#?}");
        assert!(errors[0].primary.is_some());
    }
}

#[test]
fn unit_analysis_unsupported_interfaces_generate_and_concurrent_statements_are_explicit() {
    for bytes in [
        b"entity e is port(value : in boolean); end;".as_slice(),
        b"entity e is generic(width : integer := 2); end;".as_slice(),
        b"entity e is end; architecture sim of e is signal flag : boolean; begin flag <= true; end;".as_slice(),
        b"entity e is end; architecture sim of e is begin g: if true generate begin end generate; end;".as_slice(),
        b"entity e is end; architecture sim of e is type modes is (idle, active); begin end;".as_slice(),
    ] {
        let errors = diagnostics(bytes);
        assert_eq!(errors[0].code, DiagnosticCode::UnsupportedFeature, "{errors:#?}");
        assert!(errors[0].message.contains("coverage"));
    }
}
