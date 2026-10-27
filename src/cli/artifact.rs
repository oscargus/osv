use std::{
    io::{self, Write},
    path::Path,
};

use crate::{
    artifacts::{ArtifactFormat, CodecError, DecodeLimits, encode, load, publish_new},
    diagnostics::{DiagnosticNote, display_path, render},
    source::SourceMap,
};

pub(super) fn inspect(
    input: &Path,
    output: &mut impl Write,
    errors: &mut impl Write,
) -> io::Result<u8> {
    let result = load(input, None, DecodeLimits::default())
        .and_then(|artifact| encode(&artifact, ArtifactFormat::Ron));
    match result {
        Ok(bytes) => {
            output.write_all(&bytes)?;
            writeln!(output)?;
            Ok(0)
        }
        Err(error) => report(error, &[("input", input)], errors),
    }
}

pub(super) fn convert(
    input: &Path,
    output: &Path,
    format: ArtifactFormat,
    errors: &mut impl Write,
) -> io::Result<u8> {
    let result = load(input, None, DecodeLimits::default())
        .and_then(|artifact| encode(&artifact, format))
        .and_then(|bytes| publish_new(output, &bytes));
    match result {
        Ok(()) => Ok(0),
        Err(error) => report(error, &[("input", input), ("output", output)], errors),
    }
}

fn report(error: CodecError, paths: &[(&str, &Path)], errors: &mut impl Write) -> io::Result<u8> {
    let mut diagnostic = error.diagnostic();
    for (role, path) in paths {
        diagnostic = diagnostic.with_note(DiagnosticNote::Text(format!(
            "{role}: {}",
            display_path(path)
        )));
    }
    render(&diagnostic, &SourceMap::default(), errors)?;
    Ok(diagnostic.exit_status())
}
