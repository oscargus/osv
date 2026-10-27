use std::process::{Command, Output};

use ariadne::{CharSet, Config, IndexType, Label, Report, ReportKind, Source};
use osv::diagnostics::display_path;
use vhdl_syntax::{parser, standard::VHDLStandard};

fn run(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_osv"))
        .args(arguments)
        .output()
        .unwrap()
}

#[test]
fn bootstrap_help_lists_planned_commands_honestly() {
    for arguments in [&[][..], &["--help"][..], &["-h"][..]] {
        let output = run(arguments);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        let help = String::from_utf8(output.stdout).unwrap();
        for command in ["analyze", "elaborate", "simulate", "artifact"] {
            assert!(help.contains(command), "{help}");
        }
        assert!(help.contains("not implemented yet"));
        assert!(help.contains("VHDL 2008"));
    }
}

#[test]
fn bootstrap_stage_commands_are_explicit_errors() {
    for arguments in [
        &["analyze", "not-a-source.vhd"][..],
        &["elaborate", "work.top"][..],
        &["simulate", "not-an-artifact.osve"][..],
        &["analyze", "--library", "work", "not-a-source.vhd"][..],
    ] {
        let output = run(arguments);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(error.contains("error[OSV-CLI-UNIMPLEMENTED]"), "{error}");
        assert!(error.contains(arguments[0]), "{error}");
        assert!(error.contains("not implemented"), "{error}");
    }
}

#[test]
fn bootstrap_artifact_format_options_remain_explicit_errors() {
    for format in ["binary", "ron"] {
        for (command, arguments) in [
            (
                "analyze",
                vec!["analyze", "--artifact-format", format, "not-a-source.vhd"],
            ),
            (
                "elaborate",
                vec!["elaborate", "work.top", "--artifact-format", format],
            ),
        ] {
            let output = run(&arguments);
            assert_eq!(output.status.code(), Some(2));
            assert!(output.stdout.is_empty());
            assert_eq!(
                String::from_utf8(output.stderr).unwrap(),
                format!(
                    "error[OSV-CLI-UNIMPLEMENTED]: command {command:?} is not implemented yet\n"
                ),
            );
        }
    }
}

#[test]
fn bootstrap_unknown_commands_and_options_fail() {
    for arguments in [
        &["unknown"][..],
        &["--unknown"][..],
        &["artifact", "unknown"][..],
        &["artifact"][..],
        &["artifact", "convert", "not-an-artifact.osve"][..],
    ] {
        let output = run(arguments);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(
            String::from_utf8(output.stderr)
                .unwrap()
                .contains("error[OSV-CLI-USAGE]")
        );
    }
}

#[test]
fn bootstrap_clap_subcommand_help_is_available() {
    for arguments in [
        &["analyze", "--help"][..],
        &["elaborate", "--help"][..],
        &["simulate", "--help"][..],
        &["artifact", "--help"][..],
        &["artifact", "inspect", "--help"][..],
        &["artifact", "convert", "--help"][..],
        &["help", "artifact", "inspect"][..],
    ] {
        let output = run(arguments);
        assert!(output.status.success(), "{output:?}");
        assert!(output.stderr.is_empty());
        let help = String::from_utf8(output.stdout).unwrap();
        assert!(help.contains("Usage:"), "{help}");
        if ["analyze", "elaborate", "simulate"].contains(&arguments[0]) {
            assert!(help.contains("not implemented yet"), "{help}");
        }
        assert!(!help.contains('\x1b'));
        if arguments == ["artifact", "--help"] {
            assert!(help.contains("inspect"), "{help}");
            assert!(help.contains("convert"), "{help}");
        }
    }
}

#[test]
fn bootstrap_clap_help_and_version_follow_display_precedence() {
    for arguments in [
        &["--help", "unexpected"][..],
        &["--version", "unexpected"][..],
    ] {
        let output = run(arguments);
        assert!(output.status.success());
        assert!(!output.stdout.is_empty());
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn artifact_inspect_missing_input_reports_artifact_error() {
    let output = run(&["artifact", "inspect", "not-an-artifact.osve"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(error.contains("OSV-ARTIFACT-IO"), "{error}");
    assert!(error.contains("not-an-artifact.osve"), "{error}");
}

#[test]
fn bootstrap_version_identifies_the_bootstrap() {
    let output = run(&["--version"]);
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!("osv {} (bootstrap)\n", env!("CARGO_PKG_VERSION")),
    );
}

#[test]
fn bootstrap_diagnostic_paths_render_identically() {
    fn render(path: &str) -> String {
        let label = display_path(std::path::Path::new(path));
        let source = "entity broken is\n";
        let (_, diagnostics) =
            parser::parse_with_standard(VHDLStandard::VHDL2008, source.as_bytes());
        assert_eq!(diagnostics.len(), 1);
        let span = diagnostics[0].span().clone();
        let mut output = Vec::new();
        Report::build(ReportKind::Error, (label.as_str(), span.clone()))
            .with_code("OSV-BOOTSTRAP-SYNTAX")
            .with_message("parser smoke diagnostic")
            .with_config(
                Config::default()
                    .with_color(false)
                    .with_char_set(CharSet::Ascii)
                    .with_index_type(IndexType::Byte),
            )
            .with_label(Label::new((label.as_str(), span)).with_message("expected entity ending"))
            .finish()
            .write((label.as_str(), Source::from(source)), &mut output)
            .unwrap();
        String::from_utf8(output).unwrap()
    }

    let unix = render("fixtures/broken.vhd");
    let windows = render(r"fixtures\broken.vhd");
    assert_eq!(unix, windows);
    assert!(unix.contains("fixtures/broken.vhd"));
    assert!(unix.contains("OSV-BOOTSTRAP-SYNTAX"));
    assert!(!unix.contains('\r'));
    assert!(!unix.contains('\x1b'));
}

#[test]
fn bootstrap_diagnostic_labels_preserve_prefixes() {
    for (input, expected) in [
        (r"C:\sources\broken.vhd", "C:/sources/broken.vhd"),
        (r"\\server\share\broken.vhd", "//server/share/broken.vhd"),
        ("fixtures/../broken.vhd", "fixtures/../broken.vhd"),
    ] {
        assert_eq!(display_path(std::path::Path::new(input)), expected);
    }
}
