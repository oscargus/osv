#[path = "support/artifact.rs"]
mod support;

use osv::{
    artifacts::*,
    source::{ByteSpan, SourceId},
};
use support::sample;

#[test]
fn artifact_stage_is_derived_from_owned_payload() {
    let artifact = sample();
    assert_eq!(artifact.stage(), ArtifactStage::Analysis);
    artifact.validate(ArtifactStage::Analysis).unwrap();
    assert_eq!(
        artifact.validate(ArtifactStage::Elaboration),
        Err(ArtifactError::WrongStage {
            expected: ArtifactStage::Elaboration,
            actual: ArtifactStage::Analysis,
        })
    );
}

#[test]
fn artifact_duplicate_ids_and_dangling_references_are_rejected() {
    for table in [
        TableKind::Source,
        TableKind::Unit,
        TableKind::Type,
        TableKind::Symbol,
    ] {
        let mut artifact = sample();
        match table {
            TableKind::Source => artifact.sources.push(artifact.sources[0].clone()),
            TableKind::Unit => artifact.units.push(artifact.units[0].clone()),
            TableKind::Type => artifact.types.push(artifact.types[0].clone()),
            TableKind::Symbol => artifact.symbols.push(artifact.symbols[0].clone()),
        }
        assert!(
            matches!(artifact.validate(artifact.stage()), Err(ArtifactError::DuplicateId { table: actual, .. }) if actual == table)
        );
    }
    let mut artifact = sample();
    artifact.symbols[0].type_id = Some(TypeId(99));
    assert_eq!(
        artifact.validate(artifact.stage()),
        Err(ArtifactError::DanglingReference {
            table: TableKind::Type,
            id: 99
        })
    );
}

#[test]
fn artifact_identity_preserves_semantics_but_not_table_insertion_order() {
    let mut artifact = sample();
    let mut another = artifact.symbols[0].clone();
    another.id = SymbolId(99);
    another.name = Identifier::Basic("second".into());
    artifact.symbols.push(another);
    if let ArtifactPayload::Analysis(analysis) = &mut artifact.payload {
        analysis.declarations.push(SymbolId(99));
    }
    let fingerprint = artifact.fingerprint().unwrap();
    let mut reversed = artifact.clone();
    reversed.sources.reverse();
    reversed.units.reverse();
    reversed.types.reverse();
    reversed.symbols.reverse();
    assert_eq!(reversed.fingerprint().unwrap(), fingerprint);
    assert_eq!(
        reversed.canonicalized().unwrap(),
        artifact.canonicalized().unwrap()
    );
    reversed.compiler_version = "another diagnostic build string".into();
    assert_eq!(reversed.fingerprint().unwrap(), fingerprint);
    if let ArtifactPayload::Analysis(analysis) = &mut reversed.payload {
        analysis.declarations.reverse();
    }
    assert_ne!(reversed.fingerprint().unwrap(), fingerprint);
}

#[test]
fn artifact_elaboration_has_a_concrete_root_and_secondary_architecture() {
    let mut artifact = sample();
    let mut architecture = artifact.units[0].clone();
    architecture.id = UnitId(8);
    architecture.identity.kind = UnitKind::Architecture {
        name: Identifier::Basic("sim".into()),
    };
    architecture.parent = Some(UnitId(7));
    artifact.units.push(architecture);
    artifact.payload = ArtifactPayload::Elaboration(ElaborationArtifact {
        root: UnitId(7),
        architecture: UnitId(8),
    });
    artifact.validate(ArtifactStage::Elaboration).unwrap();
    let binary = postcard::to_allocvec(&artifact).unwrap();
    let (decoded, remainder) = postcard::take_from_bytes::<ArtifactEnvelope>(&binary).unwrap();
    assert!(remainder.is_empty());
    let ron = ron::ser::to_string_pretty(&decoded, ron::ser::PrettyConfig::default()).unwrap();
    let decoded: ArtifactEnvelope = ron::from_str(&ron).unwrap();
    assert_eq!(decoded, artifact);
    assert_eq!(
        decoded.fingerprint().unwrap(),
        artifact.fingerprint().unwrap()
    );
    for name in [
        "Elaboration",
        "Vhdl2008",
        "Architecture",
        "boolean",
        "flag",
        "Scalar",
        "Boolean",
    ] {
        assert!(ron.contains(name), "{ron}");
    }
    artifact.units[1].parent = Some(UnitId(8));
    assert!(matches!(
        artifact.validate(artifact.stage()),
        Err(ArtifactError::InvalidUnit(_))
    ));
}

fn dependency(primary: &str) -> DependencyRecord {
    DependencyRecord {
        identity: UnitIdentity {
            library: Identifier::Basic("synthetic_lib".into()),
            primary: Identifier::Basic(primary.into()),
            kind: UnitKind::Package,
        },
        stage: ArtifactStage::Analysis,
        role: DependencyRole::Declaration,
        fingerprint: ContentFingerprint::of_bytes(primary.as_bytes()),
        schema_version: SCHEMA_VERSION,
        compiler_compatibility: COMPILER_COMPATIBILITY,
    }
}

#[test]
fn artifact_all_set_like_tables_have_canonical_order() {
    let mut artifact = sample();
    let mut source = artifact.sources[0].clone();
    source.file.id = SourceId(40);
    artifact.sources.push(source);
    let mut unit = artifact.units[0].clone();
    unit.id = UnitId(70);
    unit.identity.primary = Identifier::Basic("another".into());
    artifact.units.push(unit);
    let mut record = artifact.types[0].clone();
    record.id = TypeId(30);
    record.name = Identifier::Basic("another".into());
    artifact.types.push(record);
    let mut symbol = artifact.symbols[0].clone();
    symbol.id = SymbolId(50);
    symbol.name = Identifier::Basic("another".into());
    artifact.symbols.push(symbol);
    artifact.dependencies = vec![dependency("first"), dependency("second")];
    for package in ["first", "second"] {
        artifact.native_requirements.push(NativeRequirement {
            library: Identifier::Basic("synthetic_lib".into()),
            package: Identifier::Basic(package.into()),
            declarations: ContentFingerprint::of_bytes(package.as_bytes()),
            abi_version: NATIVE_ABI_VERSION,
        });
    }
    let canonical = artifact.canonicalized().unwrap();
    let fingerprint = artifact.fingerprint().unwrap();
    artifact.sources.reverse();
    artifact.units.reverse();
    artifact.types.reverse();
    artifact.symbols.reverse();
    artifact.dependencies.reverse();
    artifact.native_requirements.reverse();
    assert_eq!(artifact.canonicalized().unwrap(), canonical);
    assert_eq!(artifact.fingerprint().unwrap(), fingerprint);
    artifact.dependencies[0].fingerprint = ContentFingerprint::of_bytes(b"changed dependency");
    assert_ne!(artifact.fingerprint().unwrap(), fingerprint);
}

#[test]
fn artifact_all_reference_tables_are_checked() {
    for table in [
        TableKind::Source,
        TableKind::Unit,
        TableKind::Type,
        TableKind::Symbol,
    ] {
        let mut artifact = sample();
        match table {
            TableKind::Source => {
                artifact.symbols[0].span = Some(ByteSpan {
                    source: SourceId(99),
                    start: 0,
                    end: 0,
                })
            }
            TableKind::Unit => artifact.symbols[0].unit = UnitId(99),
            TableKind::Type => artifact.symbols[0].type_id = Some(TypeId(99)),
            TableKind::Symbol => artifact.types[0].declaration = Some(SymbolId(99)),
        }
        assert_eq!(
            artifact.validate(artifact.stage()),
            Err(ArtifactError::DanglingReference { table, id: 99 })
        );
    }
}

#[test]
fn artifact_versions_sources_and_spans_fail_explicitly() {
    let mut artifact = sample();
    artifact.schema_version += 1;
    assert!(matches!(
        artifact.validate(artifact.stage()),
        Err(ArtifactError::SchemaVersion(_))
    ));
    artifact.schema_version = SCHEMA_VERSION;
    artifact.compiler_compatibility += 1;
    assert!(matches!(
        artifact.validate(artifact.stage()),
        Err(ArtifactError::CompilerCompatibility(_))
    ));
    artifact.compiler_compatibility = COMPILER_COMPATIBILITY;
    artifact.sources[0].file.bytes.as_mut().unwrap().push(b' ');
    assert_eq!(
        artifact.validate(artifact.stage()),
        Err(ArtifactError::SourceFingerprint(SourceId(4)))
    );
    artifact.sources[0].file.bytes = None;
    artifact.symbols[0].span = Some(ByteSpan {
        source: SourceId(4),
        start: 4,
        end: 10,
    });
    artifact.validate(artifact.stage()).unwrap();
    artifact.symbols[0].span.as_mut().unwrap().start = 11;
    assert!(matches!(
        artifact.validate(artifact.stage()),
        Err(ArtifactError::InvalidSpan(_))
    ));
}

#[test]
fn artifact_secondary_units_require_exact_primary_links() {
    let primary = sample();
    let mut artifact = primary.clone();
    let mut architecture = artifact.units[0].clone();
    architecture.id = UnitId(8);
    architecture.identity.kind = UnitKind::Architecture {
        name: Identifier::Basic("sim".into()),
    };
    architecture.parent = Some(UnitId(7));
    artifact.identity = architecture.identity.clone();
    artifact.units.push(architecture);
    artifact.payload = ArtifactPayload::Analysis(AnalysisArtifact {
        unit: UnitId(8),
        declarations: Vec::new(),
    });
    assert!(matches!(
        artifact.validate(artifact.stage()),
        Err(ArtifactError::MissingDependency(_))
    ));
    let mut required = dependency("unused");
    required.identity = primary.identity.clone();
    required.fingerprint = primary.fingerprint().unwrap();
    artifact.dependencies.push(required);
    artifact.validate(artifact.stage()).unwrap();
    artifact.dependencies.push(artifact.dependencies[0].clone());
    assert!(matches!(
        artifact.validate(artifact.stage()),
        Err(ArtifactError::InvalidDependency(_))
    ));
    artifact.dependencies.pop();
    artifact.dependencies[0].compiler_compatibility += 1;
    assert!(matches!(
        artifact.validate(artifact.stage()),
        Err(ArtifactError::InvalidDependency(_))
    ));
}

#[test]
fn artifact_type_cycles_and_incompatible_native_abis_are_rejected() {
    let mut artifact = sample();
    artifact.types[0].definition = TypeDefinition::Subtype { base: TypeId(3) };
    assert_eq!(
        artifact.validate(artifact.stage()),
        Err(ArtifactError::InvalidType(3))
    );
    artifact.types[0].definition = TypeDefinition::Scalar(ScalarType::Boolean);
    artifact.native_requirements.push(NativeRequirement {
        library: Identifier::Basic("synthetic_lib".into()),
        package: Identifier::Basic("native".into()),
        declarations: ContentFingerprint::of_bytes(b"synthetic declaration set"),
        abi_version: NATIVE_ABI_VERSION + 1,
    });
    assert_eq!(
        artifact.validate(artifact.stage()),
        Err(ArtifactError::NativeRequirement)
    );
}

#[test]
fn artifact_basic_names_fold_case_but_extended_names_and_enum_order_do_not() {
    let mut artifact = sample();
    let fingerprint = artifact.fingerprint().unwrap();
    artifact.identity.primary = Identifier::Basic("SYNTHETIC".into());
    artifact.units[0].identity.primary = Identifier::Basic("SYNTHETIC".into());
    assert_eq!(artifact.fingerprint().unwrap(), fingerprint);
    artifact.symbols[0].name = Identifier::Extended("FLAG".into());
    let extended = artifact.fingerprint().unwrap();
    artifact.symbols[0].name = Identifier::Extended("flag".into());
    assert_ne!(artifact.fingerprint().unwrap(), extended);
    artifact.types[0].definition = TypeDefinition::Enumeration {
        literals: vec![
            Identifier::Basic("idle".into()),
            Identifier::Basic("active".into()),
        ],
    };
    let ordered = artifact.fingerprint().unwrap();
    if let TypeDefinition::Enumeration { literals } = &mut artifact.types[0].definition {
        literals.reverse();
    }
    assert_ne!(artifact.fingerprint().unwrap(), ordered);
    artifact.symbols[0].name = Identifier::Basic("\u{212a}".into());
    assert_eq!(artifact.fingerprint(), Err(ArtifactError::InvalidName));
}
