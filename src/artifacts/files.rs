use std::{fs::File, io::Write, path::Path};

use super::{ArtifactEnvelope, ArtifactStage, CodecError, DecodeLimits, read_from};

pub fn load(
    path: &Path,
    expected: Option<ArtifactStage>,
    limits: DecodeLimits,
) -> Result<ArtifactEnvelope, CodecError> {
    let file = File::open(path).map_err(CodecError::Io)?;
    read_from(file, expected, limits)
}

pub fn publish_new(path: &Path, bytes: &[u8]) -> Result<(), CodecError> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(CodecError::Io)?;
    temporary.write_all(bytes).map_err(CodecError::Io)?;
    temporary.as_file().sync_all().map_err(CodecError::Io)?;
    temporary
        .persist_noclobber(path)
        .map_err(|error| CodecError::Io(error.error))?;
    Ok(())
}
