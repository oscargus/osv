#[path = "support/artifact.rs"]
mod support;

use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

use osv::artifacts::*;

fn run(directory: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_osv"))
        .current_dir(directory)
        .args(arguments)
        .output()
        .unwrap()
}

fn convert(directory: &Path, input: &str, format: &str, output: &str) -> Output {
    run(
        directory,
        &[
            "artifact", "convert", input, "--to", format, "--output", output,
        ],
    )
}

#[test]
fn artifact_commands_convert_both_directions_in_fresh_processes() {
    let directory = tempfile::tempdir().unwrap();
    let original = support::sample();
    let input = encode(&original, ArtifactFormat::Binary).unwrap();
    fs::write(directory.path().join("input.ron"), &input).unwrap();
    let output = convert(directory.path(), "input.ron", "ron", "converted.bin");
    assert!(output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
    let ron = fs::read(directory.path().join("converted.bin")).unwrap();
    assert!(ron.starts_with(b"ArtifactEnvelope("));
    assert_eq!(
        decode(&ron, None, DecodeLimits::default())
            .unwrap()
            .fingerprint()
            .unwrap(),
        original.fingerprint().unwrap()
    );
    let output = convert(
        directory.path(),
        "converted.bin",
        "binary",
        "round-trip.txt",
    );
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        fs::read(directory.path().join("round-trip.txt")).unwrap(),
        input
    );
    assert_eq!(fs::read(directory.path().join("input.ron")).unwrap(), input);
}

#[test]
fn artifact_inspection_is_readable_and_does_not_change_input() {
    let directory = tempfile::tempdir().unwrap();
    for format in [ArtifactFormat::Binary, ArtifactFormat::Ron] {
        let bytes = encode(&support::sample(), format).unwrap();
        let path = directory.path().join("misleading.vhd");
        fs::write(&path, &bytes).unwrap();
        let output = run(directory.path(), &["artifact", "inspect", "misleading.vhd"]);
        assert!(output.status.success(), "{output:?}");
        assert!(output.stderr.is_empty());
        assert!(output.stdout.starts_with(b"ArtifactEnvelope("));
        assert!(String::from_utf8_lossy(&output.stdout).contains("synthetic"));
        assert_eq!(
            decode(&output.stdout, None, DecodeLimits::default()).unwrap(),
            support::sample().canonicalized().unwrap()
        );
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }
}

#[test]
fn artifact_conversion_never_overwrites_input_or_existing_output() {
    let directory = tempfile::tempdir().unwrap();
    let bytes = encode(&support::sample(), ArtifactFormat::Binary).unwrap();
    fs::write(directory.path().join("input"), &bytes).unwrap();
    fs::write(directory.path().join("existing"), b"retain this").unwrap();
    for target in ["input", "./input", "existing"] {
        let output = convert(directory.path(), "input", "ron", target);
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("OSV-ARTIFACT-IO"));
        assert_eq!(fs::read(directory.path().join("input")).unwrap(), bytes);
        assert_eq!(
            fs::read(directory.path().join("existing")).unwrap(),
            b"retain this"
        );
    }
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
}

#[test]
fn artifact_commands_reject_bad_inputs_before_publishing_output() {
    let directory = tempfile::tempdir().unwrap();
    for bytes in [
        b"not an artifact".to_vec(),
        b"OSVART\0\0".to_vec(),
        include_bytes!("fixtures/artifacts/analysis.ron")[..100].to_vec(),
    ] {
        fs::write(directory.path().join("bad"), bytes).unwrap();
        let output = convert(directory.path(), "bad", "ron", "result");
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(!directory.path().join("result").exists());
    }
    let mut artifact = support::sample();
    artifact.schema_version += 1;
    let bytes = ron::ser::to_string_pretty(&artifact, ron::ser::PrettyConfig::default()).unwrap();
    fs::write(directory.path().join("incompatible"), bytes).unwrap();
    let output = convert(directory.path(), "incompatible", "binary", "result");
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("OSV-ARTIFACT-COMPATIBILITY"));
    assert!(!directory.path().join("result").exists());
}

#[test]
fn artifact_conversion_argument_errors_remain_usage_errors() {
    let directory = tempfile::tempdir().unwrap();
    for arguments in [
        &["artifact", "convert", "input", "--to", "ron"][..],
        &["artifact", "convert", "input", "--output", "result"][..],
        &[
            "artifact", "convert", "input", "--to", "json", "--output", "result",
        ][..],
        &["artifact", "inspect"][..],
    ] {
        let output = run(directory.path(), arguments);
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("OSV-CLI-USAGE"));
        assert!(!directory.path().join("result").exists());
    }
}

#[test]
fn artifact_publication_failure_leaves_no_temporary_files() {
    let directory = tempfile::tempdir().unwrap();
    let input = encode(&support::sample(), ArtifactFormat::Binary).unwrap();
    fs::write(directory.path().join("input"), &input).unwrap();
    let output = convert(directory.path(), "input", "ron", "missing-parent/result");
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
}
