use std::{
    io::{self, Write},
    path::Path,
};

use osv::{
    diagnostics::{
        ContextKind, Diagnostic, DiagnosticCode, DiagnosticContext, DiagnosticNote,
        DiagnosticStage, Severity, exit_status, render,
    },
    source::{ByteSpan, SourceFile, SourceId, SourceMap},
};
use vhdl_syntax::{parser, standard::VHDLStandard};

fn rendered(diagnostic: &Diagnostic, sources: &SourceMap) -> String {
    let mut output = Vec::new();
    render(diagnostic, sources, &mut output).unwrap();
    let output = String::from_utf8(output).unwrap();
    assert!(!output.contains('\r'));
    assert!(!output.contains('\x1b'));
    output
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn expected_output(snapshot: &str) -> String {
    ron::from_str(&snapshot.replace("\r\n", "\n")).unwrap()
}

fn type_mismatch() -> (SourceMap, Diagnostic) {
    let mut sources = SourceMap::default();
    let target = sources
        .add(
            Path::new(r"fixtures\target.vhd"),
            b"signal caf\xe9 : boolean := 42;\n",
        )
        .unwrap();
    let declaration = sources
        .add(
            Path::new("fixtures/value.vhd"),
            b"constant supplied : integer := 42;\n",
        )
        .unwrap();
    let diagnostic = Diagnostic::new(
        DiagnosticCode::TypeMismatch,
        DiagnosticStage::Elaboration,
        Severity::Error,
        "initializer type mismatch",
    )
    .with_primary(
        ByteSpan {
            source: target,
            start: 25,
            end: 27,
        },
        "initializer has type integer",
    )
    .with_related(
        ByteSpan {
            source: declaration,
            start: 20,
            end: 27,
        },
        "value declared with type integer",
    )
    .with_note(DiagnosticNote::ExpectedActual {
        expected: "boolean".into(),
        actual: "integer range 0 to 255".into(),
    })
    .with_note(DiagnosticNote::Text(
        "no implicit integer-to-boolean conversion is permitted".into(),
    ))
    .with_context(DiagnosticContext {
        kind: ContextKind::Instance,
        name: "work.tb.u0".into(),
        span: None,
    })
    .with_context(DiagnosticContext {
        kind: ContextKind::Call,
        name: "work.values.choose".into(),
        span: Some(ByteSpan {
            source: declaration,
            start: 9,
            end: 17,
        }),
    });
    (sources, diagnostic)
}

#[test]
fn diagnostics_syntax_uses_the_parser_byte_span() {
    let bytes = b"-- caf\xe9\r\nentity broken is\r\n";
    let (_, errors) = parser::parse_with_standard(VHDLStandard::VHDL2008, bytes);
    assert_eq!(errors.len(), 1);
    let range = errors[0].span();
    let mut sources = SourceMap::default();
    let source = sources
        .add(Path::new("fixtures/broken.vhd"), bytes)
        .unwrap();
    let diagnostic = Diagnostic::new(
        DiagnosticCode::Syntax,
        DiagnosticStage::Analysis,
        Severity::Error,
        "expected entity ending",
    )
    .with_primary(
        ByteSpan {
            source,
            start: u32::try_from(range.start).unwrap(),
            end: u32::try_from(range.end).unwrap(),
        },
        "entity declaration is incomplete",
    );
    assert_eq!(
        diagnostic.primary.as_ref().unwrap().span.start,
        diagnostic.primary.as_ref().unwrap().span.end
    );
    let output = rendered(&diagnostic, &sources);
    assert!(output.contains("OSV-SYNTAX"));
    assert!(output.contains("entity declaration is incomplete"));
    assert_eq!(
        output,
        expected_output(include_str!("snapshots/diagnostics/syntax.ron"))
    );
}

#[test]
fn diagnostics_type_mismatch_retains_related_types_and_nested_context() {
    let (sources, diagnostic) = type_mismatch();
    assert_eq!(diagnostic.related.len(), 1);
    assert_eq!(diagnostic.context.len(), 2);
    assert_eq!(diagnostic.severity, Severity::Error);
    let output = rendered(&diagnostic, &sources);
    assert!(output.contains("caf\u{e9}"));
    assert!(output.contains("expected type: boolean"));
    assert!(output.contains("actual type: integer range 0 to 255"));
    assert!(output.contains("fixtures/value.vhd"));
    assert_eq!(
        output,
        expected_output(include_str!("snapshots/diagnostics/type-mismatch.ron"))
    );
}

#[test]
fn diagnostics_missing_sources_keep_names_offsets_and_notes() {
    let mut sources = SourceMap::default();
    sources
        .insert(SourceFile::unavailable(
            SourceId(4),
            Path::new(r"missing\design.vhd"),
        ))
        .unwrap();
    let diagnostic = Diagnostic::new(
        DiagnosticCode::TypeMismatch,
        DiagnosticStage::Simulation,
        Severity::Error,
        "saved design type mismatch",
    )
    .with_primary(
        ByteSpan {
            source: SourceId(4),
            start: 12,
            end: 15,
        },
        "stored assignment",
    )
    .with_related(
        ByteSpan {
            source: SourceId(99),
            start: 20,
            end: 24,
        },
        "original declaration",
    )
    .with_note(DiagnosticNote::ExpectedActual {
        expected: "bit_vector(7 downto 0)".into(),
        actual: "bit_vector(3 downto 0)".into(),
    })
    .with_context(DiagnosticContext {
        kind: ContextKind::Instance,
        name: "work.tb.u1".into(),
        span: None,
    });
    let output = rendered(&diagnostic, &sources);
    assert!(output.contains("source unavailable: missing/design.vhd:bytes 12..15"));
    assert!(output.contains("<source 99>:bytes 20..24"));
    assert!(output.contains("expected type: bit_vector(7 downto 0)"));
    assert_eq!(
        output,
        expected_output(include_str!("snapshots/diagnostics/unavailable.ron"))
    );
}

#[test]
fn diagnostics_snapshot_checkouts_support_lf_and_crlf() {
    for snapshot in [
        include_str!("snapshots/diagnostics/syntax.ron"),
        include_str!("snapshots/diagnostics/type-mismatch.ron"),
        include_str!("snapshots/diagnostics/unavailable.ron"),
    ] {
        let unix = snapshot.replace("\r\n", "\n");
        let windows = unix.replace('\n', "\r\n");
        assert_eq!(expected_output(&unix), expected_output(&windows));
    }
}

#[test]
fn diagnostics_exit_status_policy_is_stable() {
    assert_eq!(exit_status(&[]), 0);
    for stage in [
        DiagnosticStage::Analysis,
        DiagnosticStage::Elaboration,
        DiagnosticStage::Simulation,
        DiagnosticStage::Artifact,
        DiagnosticStage::CommandLine,
    ] {
        for severity in [Severity::Advice, Severity::Warning, Severity::Error] {
            let diagnostic = Diagnostic::new(
                DiagnosticCode::TypeMismatch,
                stage,
                severity,
                "synthetic diagnostic",
            );
            let expected = match (severity, stage) {
                (Severity::Error, DiagnosticStage::CommandLine) => 2,
                (Severity::Error, _) => 1,
                _ => 0,
            };
            assert_eq!(diagnostic.exit_status(), expected);
            assert_eq!(exit_status(&[diagnostic]), expected);
        }
    }
    let warning = Diagnostic::new(
        DiagnosticCode::Syntax,
        DiagnosticStage::Analysis,
        Severity::Warning,
        "warning",
    );
    let error = Diagnostic::new(
        DiagnosticCode::Syntax,
        DiagnosticStage::Analysis,
        Severity::Error,
        "error",
    );
    let usage = Diagnostic::new(
        DiagnosticCode::CliUsage,
        DiagnosticStage::CommandLine,
        Severity::Error,
        "usage",
    );
    assert_eq!(exit_status(&[warning, error, usage]), 2);
    assert_eq!(DiagnosticCode::CliUsage.to_string(), "OSV-CLI-USAGE");
    assert_eq!(
        DiagnosticCode::CliUnimplemented.to_string(),
        "OSV-CLI-UNIMPLEMENTED"
    );
    assert_eq!(DiagnosticCode::CliIo.to_string(), "OSV-CLI-IO");
}

#[test]
fn diagnostics_preserve_records_and_rendering_in_both_codecs() {
    let (sources, diagnostic) = type_mismatch();
    let original = (
        sources.iter().cloned().collect::<Vec<_>>(),
        diagnostic.clone(),
    );
    let binary = postcard::to_allocvec(&original).unwrap();
    let from_binary: (Vec<SourceFile>, Diagnostic) = postcard::from_bytes(&binary).unwrap();
    let ron = ron::ser::to_string_pretty(&from_binary, ron::ser::PrettyConfig::default()).unwrap();
    let from_ron: (Vec<SourceFile>, Diagnostic) = ron::from_str(&ron).unwrap();
    assert_eq!(from_ron, original);
    assert_eq!(postcard::to_allocvec(&from_ron).unwrap(), binary);
    for name in [
        "TypeMismatch",
        "Elaboration",
        "ExpectedActual",
        "Instance",
        "Call",
        "Latin1",
    ] {
        assert!(ron.contains(name), "{ron}");
    }
    let mut restored = SourceMap::default();
    for source in from_ron.0 {
        restored.insert(source).unwrap();
    }
    assert_eq!(
        rendered(&from_ron.1, &restored),
        rendered(&diagnostic, &sources)
    );
}

#[test]
fn diagnostics_normalize_restored_path_labels_without_changing_ids() {
    let (sources, diagnostic) = type_mismatch();
    let mut restored = SourceMap::default();
    for mut source in sources.iter().cloned() {
        source.display_name = source.display_name.replace('/', "\\");
        let id = source.id;
        restored.insert(source).unwrap();
        assert_eq!(restored.get(id).unwrap().id, id);
    }
    assert_eq!(
        rendered(&diagnostic, &sources),
        rendered(&diagnostic, &restored)
    );
}

#[test]
fn diagnostics_reject_invalid_spans_without_panicking_or_losing_messages() {
    let mut sources = SourceMap::default();
    let source = sources
        .add(Path::new("bounds.vhd"), b"signal value : bit;\n")
        .unwrap();
    for (start, end) in [(12, 4), (0, u32::MAX), (u32::MAX, u32::MAX)] {
        let diagnostic = Diagnostic::new(
            DiagnosticCode::Syntax,
            DiagnosticStage::Analysis,
            Severity::Error,
            "invalid stored source location",
        )
        .with_primary(ByteSpan { source, start, end }, "primary location retained")
        .with_related(
            ByteSpan {
                source,
                start: 0,
                end: 6,
            },
            "valid related location retained",
        );
        let output = rendered(&diagnostic, &sources);
        assert!(output.contains(&format!("invalid byte span: bounds.vhd:{start}..{end}")));
        assert!(output.contains("primary location retained"));
        assert!(output.contains("valid related location retained"));
    }
}

#[test]
fn diagnostics_render_empty_and_eof_spans() {
    for bytes in [&b""[..], &b"end entity;"[..], &b"end entity;\r\n"[..]] {
        let mut sources = SourceMap::default();
        let source = sources.add(Path::new("eof.vhd"), bytes).unwrap();
        let end = u32::try_from(bytes.len()).unwrap();
        let diagnostic = Diagnostic::new(
            DiagnosticCode::Syntax,
            DiagnosticStage::Analysis,
            Severity::Error,
            "unexpected end of source",
        )
        .with_primary(
            ByteSpan {
                source,
                start: end,
                end,
            },
            "expected declaration",
        );
        let output = rendered(&diagnostic, &sources);
        assert!(output.contains("expected declaration"));
        assert!(output.contains("eof.vhd"));
        assert!(!output.contains("invalid byte span"));
    }
}

#[test]
fn diagnostics_missing_primary_keeps_available_related_source() {
    let (sources, mut diagnostic) = type_mismatch();
    diagnostic.primary.as_mut().unwrap().span.source = SourceId(99);
    let output = rendered(&diagnostic, &sources);
    assert!(
        output
            .contains("source unavailable: <source 99>:bytes 25..27: initializer has type integer")
    );
    assert!(output.contains("fixtures/value.vhd"));
    assert!(output.contains("value declared with type integer"));
    assert!(output.contains("expected type: boolean"));
}

#[test]
fn diagnostics_do_not_read_existing_paths_for_unavailable_sources() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    assert!(path.is_file());
    let mut sources = SourceMap::default();
    sources
        .insert(SourceFile::unavailable(SourceId(1), &path))
        .unwrap();
    let diagnostic = Diagnostic::new(
        DiagnosticCode::Syntax,
        DiagnosticStage::Artifact,
        Severity::Error,
        "source bytes were not saved",
    )
    .with_primary(
        ByteSpan {
            source: SourceId(1),
            start: 0,
            end: 1,
        },
        "stored location",
    );
    let output = rendered(&diagnostic, &sources);
    assert!(output.contains("source unavailable:"));
    assert!(!output.contains("[package]"));
}

#[test]
fn diagnostics_sourceless_severities_keep_notes() {
    for (severity, name) in [
        (Severity::Advice, "Advice"),
        (Severity::Warning, "Warning"),
        (Severity::Error, "Error"),
    ] {
        let diagnostic = Diagnostic::new(
            DiagnosticCode::Syntax,
            DiagnosticStage::Analysis,
            severity,
            "message without a location",
        )
        .with_note(DiagnosticNote::Text("retained explanatory note".into()));
        let output = rendered(&diagnostic, &SourceMap::default());
        assert!(output.starts_with(&format!("[OSV-SYNTAX] {name}:")));
        assert!(output.contains("retained explanatory note"));
    }
}

#[test]
fn diagnostics_propagate_output_failures() {
    struct FailedOutput;

    impl Write for FailedOutput {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("output unavailable"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    let (sources, diagnostic) = type_mismatch();
    let error = render(&diagnostic, &sources, &mut FailedOutput).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::Other);
    assert_eq!(error.to_string(), "output unavailable");
}
