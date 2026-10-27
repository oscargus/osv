#[path = "support/artifact.rs"]
mod support;

use std::io::{self, Read};

use osv::{
    artifacts::*,
    diagnostics::{DiagnosticCode, DiagnosticStage},
};
use serde::Serialize;

fn sample(stage: ArtifactStage) -> ArtifactEnvelope {
    let mut artifact = support::sample();
    if stage == ArtifactStage::Elaboration {
        let mut architecture = artifact.units[0].clone();
        architecture.id = UnitId(8);
        architecture.identity.kind = UnitKind::Architecture {
            name: Identifier::Basic("sim".into()),
        };
        architecture.parent = Some(UnitId(7));
        artifact.units.push(architecture);
        artifact.scopes.push(ScopeRecord {
            id: ScopeId(2),
            parent: Some(ScopeId(1)),
            kind: ScopeKind::Unit(UnitId(8)),
        });
        artifact.payload = ArtifactPayload::Elaboration(ElaborationArtifact {
            root: UnitId(7),
            architecture: UnitId(8),
        });
    }
    artifact
}

#[test]
fn artifact_codec_rejects_bad_headers_before_payload() {
    assert_eq!(ArtifactFormat::default(), ArtifactFormat::Binary);
    for input in [&b""[..], b"OSVART\0\0", b"not an artifact"] {
        assert!(decode(input, None, DecodeLimits::default()).is_err());
    }
    let mut header = b"OSVART\0\0".to_vec();
    header.extend(1u32.to_le_bytes());
    header.push(1);
    header.extend(u64::MAX.to_le_bytes());
    assert!(decode(&header, None, DecodeLimits::default()).is_err());
}

#[test]
fn artifact_codecs_round_trip_both_stages_and_all_conversions() {
    for stage in [ArtifactStage::Analysis, ArtifactStage::Elaboration] {
        let artifact = sample(stage);
        let canonical = artifact.canonicalized().unwrap();
        let identity = artifact.fingerprint().unwrap();
        for from in [ArtifactFormat::Binary, ArtifactFormat::Ron] {
            let encoded = encode(&artifact, from).unwrap();
            let loaded = decode(&encoded, Some(stage), DecodeLimits::default()).unwrap();
            assert_eq!(loaded, canonical);
            assert_eq!(encode(&loaded, from).unwrap(), encoded);
            for to in [ArtifactFormat::Binary, ArtifactFormat::Ron] {
                let converted = encode(&loaded, to).unwrap();
                let restored =
                    read_from(&converted[..], Some(stage), DecodeLimits::default()).unwrap();
                assert_eq!(restored, canonical);
                assert_eq!(restored.fingerprint().unwrap(), identity);
            }
        }
    }
}

#[test]
fn artifact_codec_limits_apply_to_both_formats() {
    let artifact = sample(ArtifactStage::Analysis);
    for format in [ArtifactFormat::Binary, ArtifactFormat::Ron] {
        let bytes = encode(&artifact, format).unwrap();
        for limit in [
            ResourceLimit::InputBytes,
            ResourceLimit::CollectionEntries,
            ResourceLimit::StringBytes,
            ResourceLimit::TotalEntries,
            ResourceLimit::AllocationBytes,
            ResourceLimit::Depth,
        ] {
            let mut limits = DecodeLimits::default();
            match limit {
                ResourceLimit::InputBytes => limits.max_input_bytes = bytes.len() - 1,
                ResourceLimit::CollectionEntries => limits.max_collection_entries = 1,
                ResourceLimit::StringBytes => limits.max_string_bytes = 2,
                ResourceLimit::TotalEntries => limits.max_total_entries = 1,
                ResourceLimit::AllocationBytes => limits.max_allocation_bytes = 0,
                ResourceLimit::Depth => limits.max_depth = 1,
            }
            assert!(
                matches!(decode(&bytes, None, limits), Err(CodecError::Limit(actual)) if actual == limit),
                "{format:?} {limit:?}"
            );
        }
    }
}

#[test]
fn artifact_codec_limit_boundaries_do_not_charge_schema_names_as_data() {
    let artifact = sample(ArtifactStage::Analysis);
    let source_length = artifact.sources[0].file.bytes.as_ref().unwrap().len();
    for format in [ArtifactFormat::Binary, ArtifactFormat::Ron] {
        let bytes = encode(&artifact, format).unwrap();
        let mut limits = DecodeLimits {
            max_input_bytes: bytes.len(),
            max_string_bytes: "synthetic.vhd".len(),
            max_collection_entries: source_length,
            ..DecodeLimits::default()
        };
        decode(&bytes, None, limits).unwrap();
        limits.max_collection_entries -= 1;
        assert!(matches!(
            decode(&bytes, None, limits),
            Err(CodecError::Limit(ResourceLimit::CollectionEntries))
        ));
    }
}

fn framed(payload: &[u8], stage: u8) -> Vec<u8> {
    let mut bytes = b"OSVART\0\0".to_vec();
    bytes.extend(SCHEMA_VERSION.to_le_bytes());
    bytes.push(stage);
    bytes.extend(u64::try_from(payload.len()).unwrap().to_le_bytes());
    bytes.extend(payload);
    bytes
}

#[test]
fn artifact_codec_hostile_collection_length_is_rejected_before_elements() {
    #[derive(Serialize)]
    struct Prefix<'value> {
        schema_version: u32,
        compiler_compatibility: u32,
        compiler_version: &'value str,
        language: LanguageRevision,
        identity: &'value UnitIdentity,
    }
    let artifact = sample(ArtifactStage::Analysis);
    let mut prefix = postcard::to_allocvec(&Prefix {
        schema_version: SCHEMA_VERSION,
        compiler_compatibility: COMPILER_COMPATIBILITY,
        compiler_version: "test",
        language: LanguageRevision::Vhdl2008,
        identity: &artifact.identity,
    })
    .unwrap();
    prefix.extend(postcard::to_allocvec(&u64::MAX).unwrap());
    let error = decode(&framed(&prefix, 1), None, DecodeLimits::default()).unwrap_err();
    assert!(
        matches!(
            error,
            CodecError::Limit(ResourceLimit::CollectionEntries) | CodecError::Malformed(_)
        ),
        "{error:?}"
    );
}

#[test]
fn artifact_codec_truncation_trailing_data_and_invalid_fields_fail() {
    let artifact = sample(ArtifactStage::Analysis);
    let binary = encode(&artifact, ArtifactFormat::Binary).unwrap();
    for length in 0..binary.len() {
        assert!(
            decode(&binary[..length], None, DecodeLimits::default()).is_err(),
            "prefix {length}"
        );
    }
    let mut payload = postcard::to_allocvec(&artifact).unwrap();
    payload.push(0);
    assert!(matches!(
        decode(&framed(&payload, 1), None, DecodeLimits::default()),
        Err(CodecError::Framing(_))
    ));
    let ron = String::from_utf8(encode(&artifact, ArtifactFormat::Ron).unwrap()).unwrap();
    for invalid in [
        ron[..ron.len() / 2].to_string(),
        format!("{ron}\ntrailing"),
        ron.replace("Vhdl2008", "Vhdl2019"),
        ron.replace(
            &format!("schema_version: {SCHEMA_VERSION}"),
            "schema_version: 999",
        ),
        ron.replace("compiler_version:", "unknown_field:"),
        ron.replace("display_name:", "unexpected: 0, display_name:"),
    ] {
        assert!(
            decode(invalid.as_bytes(), None, DecodeLimits::default()).is_err(),
            "{invalid}"
        );
    }
}

#[test]
fn artifact_codec_header_and_payload_stage_and_versions_must_agree() {
    let artifact = sample(ArtifactStage::Analysis);
    let mut binary = encode(&artifact, ArtifactFormat::Binary).unwrap();
    binary[12] = 2;
    assert!(matches!(
        decode(&binary, None, DecodeLimits::default()),
        Err(CodecError::Validation(ArtifactError::WrongStage { .. }))
    ));
    binary[12] = 255;
    assert!(matches!(
        decode(&binary, None, DecodeLimits::default()),
        Err(CodecError::Framing(_))
    ));
    binary[12] = 1;
    binary[8..12].copy_from_slice(&999u32.to_le_bytes());
    assert!(matches!(
        decode(&binary, None, DecodeLimits::default()),
        Err(CodecError::Validation(ArtifactError::SchemaVersion(999)))
    ));
    for format in [ArtifactFormat::Binary, ArtifactFormat::Ron] {
        let bytes = encode(&artifact, format).unwrap();
        assert!(matches!(
            decode(
                &bytes,
                Some(ArtifactStage::Elaboration),
                DecodeLimits::default()
            ),
            Err(CodecError::Validation(ArtifactError::WrongStage { .. }))
        ));
    }
}

#[test]
fn artifact_codec_failures_are_shared_structured_diagnostics() {
    let artifact = sample(ArtifactStage::Analysis);
    let bytes = encode(&artifact, ArtifactFormat::Binary).unwrap();
    let error = decode(
        &bytes,
        Some(ArtifactStage::Elaboration),
        DecodeLimits::default(),
    )
    .unwrap_err();
    let diagnostic = error.diagnostic();
    assert_eq!(diagnostic.stage, DiagnosticStage::Artifact);
    assert_eq!(diagnostic.code, DiagnosticCode::ArtifactCompatibility);
    assert_eq!(diagnostic.exit_status(), 1);
    let diagnostic = CodecError::Limit(ResourceLimit::InputBytes).diagnostic();
    assert_eq!(diagnostic.code.to_string(), "OSV-ARTIFACT-LIMIT");
}

#[test]
fn artifact_codec_reader_is_bounded_and_propagates_io_failures() {
    struct FailedReader;
    impl Read for FailedReader {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("fixture read failed"))
        }
    }
    let error = read_from(FailedReader, None, DecodeLimits::default()).unwrap_err();
    assert_eq!(error.diagnostic().code, DiagnosticCode::ArtifactIo);
    let mut reader = io::Cursor::new([0u8; 100]);
    let error = read_from(
        &mut reader,
        None,
        DecodeLimits {
            max_input_bytes: 10,
            ..DecodeLimits::default()
        },
    )
    .unwrap_err();
    assert!(matches!(
        error,
        CodecError::Limit(ResourceLimit::InputBytes)
    ));
    assert_eq!(reader.position(), 11);
}

#[test]
fn artifact_codec_ron_fixture_has_named_readable_structure() {
    let bytes = include_bytes!("fixtures/artifacts/analysis.ron");
    let artifact = decode(
        bytes,
        Some(ArtifactStage::Analysis),
        DecodeLimits::default(),
    )
    .unwrap();
    assert_eq!(artifact.symbols[0].name, Identifier::Basic("flag".into()));
    assert_eq!(
        artifact.types[0].definition,
        TypeDefinition::Scalar(ScalarType::Boolean)
    );
    let output = String::from_utf8(encode(&artifact, ArtifactFormat::Ron).unwrap()).unwrap();
    assert!(output.starts_with("ArtifactEnvelope("));
    for name in [
        "Analysis",
        "Boolean",
        "synthetic_fixture",
        "flag",
        "UnitId",
        "SymbolId",
    ] {
        assert!(output.contains(name), "{output}");
    }
}

#[test]
fn artifact_codec_ron_parser_workspace_is_reserved_before_token_allocation() {
    let text = include_bytes!("fixtures/artifacts/analysis.ron");
    let limits = DecodeLimits {
        max_allocation_bytes: text.len(),
        ..DecodeLimits::default()
    };
    assert!(matches!(
        decode(text, None, limits),
        Err(CodecError::Limit(ResourceLimit::AllocationBytes))
    ));
}
