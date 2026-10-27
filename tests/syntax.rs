#[allow(dead_code)]
#[path = "support/fixture.rs"]
mod fixture;
#[path = "support/golden.rs"]
mod golden;

use std::path::Path;

use osv::{
    diagnostics::{DiagnosticCode, DiagnosticStage, exit_status},
    source::{ByteSpan, SourceFile, SourceId, SourceMap},
    syntax::{self, NodeKind},
};

#[derive(Debug, serde::Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Summary {
    parsed: bool,
    design_units: u32,
}

#[test]
fn syntax_adapter_preserves_latin1_crlf_error_byte_span() {
    let source = SourceFile::new(
        SourceId(4),
        Path::new("broken.vhd"),
        b"-- caf\xe9\r\nentity broken is\r\n",
    );
    let errors = syntax::parse(&source).unwrap_err();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, DiagnosticCode::Syntax);
    assert_eq!(errors[0].stage, DiagnosticStage::Analysis);
    assert_eq!(
        errors[0].primary.as_ref().unwrap().span,
        ByteSpan {
            source: SourceId(4),
            start: 25,
            end: 25
        }
    );
}

#[test]
fn syntax_adapter_fixture_results_have_explicit_stages_codes_and_status() {
    for name in ["syntax/valid.ron", "syntax/invalid.ron", "syntax/psl.ron"] {
        let fixture = fixture::Fixture::<Summary>::load(name).unwrap();
        let source = fixture.sources.iter().next().unwrap();
        let (summary, diagnostics) = match syntax::parse(source) {
            Ok(parsed) => (
                Summary {
                    parsed: true,
                    design_units: u32::try_from(parsed.design_units().count()).unwrap(),
                },
                Vec::new(),
            ),
            Err(diagnostics) => (
                Summary {
                    parsed: false,
                    design_units: 0,
                },
                diagnostics,
            ),
        };
        let output = diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        fixture
            .check(&fixture::Observation {
                kind: fixture::FixtureKind::ParserProbe,
                stage: DiagnosticStage::Analysis,
                exit_status: exit_status(&diagnostics),
                diagnostics,
                result: summary,
                output,
            })
            .unwrap();
    }
}

#[test]
fn syntax_adapter_exposes_units_expressions_tokens_and_lossless_bytes() {
    let source = SourceFile::new(
        SourceId(42),
        Path::new("mixed.vhd"),
        include_bytes!("fixtures/vhdl/syntax-mixed.vhd"),
    );
    let parsed = syntax::parse(&source).unwrap();
    assert_eq!(parsed.source_id(), SourceId(42));
    assert_eq!(parsed.design_units().count(), 3);
    let kinds = parsed
        .root()
        .descendants()
        .map(|node| node.kind())
        .collect::<Vec<_>>();
    for kind in [
        NodeKind::EntityDeclaration,
        NodeKind::ArchitectureBody,
        NodeKind::PrimaryUnitPackageDeclaration,
        NodeKind::BinaryExpression,
        NodeKind::LiteralExpression,
    ] {
        assert!(kinds.contains(&kind), "missing {kind:?}");
    }
    let root = parsed.root();
    assert!(root.tokens().count() > 0);
    assert_eq!(root.children().count(), 3);
    for node in root.descendants() {
        assert!(
            node.span()
                .unwrap()
                .checked_range(source.bytes.as_ref().unwrap().len())
                .is_some()
        );
    }
    let tokens = root.all_tokens().collect::<Vec<_>>();
    assert!(tokens.iter().any(|token| token.text() == "+"));
    for token in tokens {
        assert!(
            token
                .span()
                .unwrap()
                .checked_range(source.bytes.as_ref().unwrap().len())
                .is_some()
        );
        let _ = token.kind();
    }
    let mut bytes = Vec::new();
    parsed.write_to(&mut bytes).unwrap();
    assert_eq!(bytes, *source.bytes.as_ref().unwrap());
    assert_eq!(parsed.span().end, u32::try_from(bytes.len()).unwrap());
}

#[test]
fn syntax_adapter_does_not_perform_name_or_type_analysis() {
    let source = SourceFile::new(SourceId(0), Path::new("semantic.vhd"), b"entity names is end; architecture sim of names is signal missing : unknown_type; signal wrong : boolean := 42; begin end;");
    assert!(syntax::parse(&source).is_ok());
}

#[test]
fn syntax_adapter_rejects_2019_mode_views_in_2008() {
    let source = SourceFile::new(
        SourceId(0),
        Path::new("mode-view.vhd"),
        include_bytes!("fixtures/vhdl/syntax-2019.vhd"),
    );
    let errors = syntax::parse(&source).unwrap_err();
    assert!(!errors.is_empty());
    assert!(
        errors
            .iter()
            .all(|error| error.code == DiagnosticCode::Syntax
                && error.stage == DiagnosticStage::Analysis)
    );
}

#[test]
fn syntax_adapter_psl_comments_are_explicit_errors_not_ignored_assertions() {
    for bytes in [
        b"-- psl assert never false;\n".as_slice(),
        b"entity e is end; -- PSL assert always true;\n".as_slice(),
    ] {
        let source = SourceFile::new(SourceId(0), Path::new("psl.vhd"), bytes);
        let errors = syntax::parse(&source).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.code == DiagnosticCode::PslUnsupported)
        );
        assert!(
            errors
                .iter()
                .any(|error| format!("{:?}", error.notes).contains("L067"))
        );
    }
    let source = SourceFile::new(
        SourceId(0),
        Path::new("ordinary.vhd"),
        b"-- pslike is not a directive\nentity e is end;\n",
    );
    assert!(syntax::parse(&source).is_ok());
}

#[test]
fn syntax_adapter_renders_real_errors_and_missing_sources_through_shared_api() {
    let mut sources = SourceMap::default();
    let id = sources
        .add(Path::new("fixtures/broken.vhd"), b"entity broken is\n")
        .unwrap();
    let errors = syntax::parse(sources.get(id).unwrap()).unwrap_err();
    let output = golden::rendered(&errors[0], &sources);
    let snapshot = ron::to_string(&output).unwrap();
    assert_eq!(golden::expected_output(&snapshot), output);
    assert!(output.contains("OSV-SYNTAX"));
    assert!(output.contains("expected EntityDeclarationEpilogue"));
    let unavailable = SourceFile::unavailable(SourceId(99), Path::new("missing.vhd"));
    let errors = syntax::parse(&unavailable).unwrap_err();
    assert_eq!(errors[0].code, DiagnosticCode::SourceUnavailable);
    assert!(golden::rendered(&errors[0], &sources).contains("source unavailable"));
}
