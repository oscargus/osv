use std::{
    error::Error,
    fmt,
    io::{self, Read},
};

use serde::Deserialize;

use crate::diagnostics::{Diagnostic, DiagnosticCode, DiagnosticStage, Severity};

use super::bounded::{Bounded, Budget};
use super::{ArtifactEnvelope, ArtifactError, ArtifactStage, SCHEMA_VERSION};

const MAGIC: &[u8; 8] = b"OSVART\0\0";
const HEADER_BYTES: usize = 21;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ArtifactFormat {
    #[default]
    Binary,
    Ron,
}

#[derive(Clone, Copy, Debug)]
pub struct DecodeLimits {
    pub max_input_bytes: usize,
    pub max_collection_entries: usize,
    pub max_string_bytes: usize,
    pub max_total_entries: usize,
    pub max_allocation_bytes: usize,
    pub max_depth: usize,
}

impl Default for DecodeLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: 64 * 1024 * 1024,
            max_collection_entries: 1_000_000,
            max_string_bytes: 1024 * 1024,
            max_total_entries: 4_000_000,
            max_allocation_bytes: 256 * 1024 * 1024,
            max_depth: 128,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceLimit {
    InputBytes,
    CollectionEntries,
    StringBytes,
    TotalEntries,
    AllocationBytes,
    Depth,
}

#[derive(Debug)]
pub enum CodecError {
    Framing(String),
    Limit(ResourceLimit),
    Malformed(String),
    Validation(ArtifactError),
    Io(io::Error),
}

impl fmt::Display for CodecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Framing(message) => write!(formatter, "invalid artifact framing: {message}"),
            Self::Limit(limit) => write!(formatter, "artifact input limit exceeded: {limit:?}"),
            Self::Malformed(message) => write!(formatter, "malformed artifact: {message}"),
            Self::Validation(error) => error.fmt(formatter),
            Self::Io(error) => write!(formatter, "artifact I/O failed: {error}"),
        }
    }
}

impl Error for CodecError {}

impl CodecError {
    pub fn diagnostic(&self) -> Diagnostic {
        let code = match self {
            Self::Limit(_) => DiagnosticCode::ArtifactLimit,
            Self::Validation(
                ArtifactError::SchemaVersion(_)
                | ArtifactError::CompilerCompatibility(_)
                | ArtifactError::WrongStage { .. }
                | ArtifactError::NativeRequirement
                | ArtifactError::InvalidDependency(_),
            ) => DiagnosticCode::ArtifactCompatibility,
            Self::Validation(_) => DiagnosticCode::ArtifactValidation,
            Self::Io(_) => DiagnosticCode::ArtifactIo,
            Self::Framing(_) | Self::Malformed(_) => DiagnosticCode::ArtifactFormat,
        };
        Diagnostic::new(
            code,
            DiagnosticStage::Artifact,
            Severity::Error,
            self.to_string(),
        )
    }
}

pub fn encode(artifact: &ArtifactEnvelope, format: ArtifactFormat) -> Result<Vec<u8>, CodecError> {
    let artifact = artifact.canonicalized().map_err(CodecError::Validation)?;
    match format {
        ArtifactFormat::Binary => {
            let payload = postcard::to_allocvec(&artifact)
                .map_err(|error| CodecError::Malformed(error.to_string()))?;
            let mut output = MAGIC.to_vec();
            output.extend(SCHEMA_VERSION.to_le_bytes());
            output.push(match artifact.stage() {
                ArtifactStage::Analysis => 1,
                ArtifactStage::Elaboration => 2,
            });
            output.extend(
                u64::try_from(payload.len())
                    .map_err(|_| CodecError::Limit(ResourceLimit::InputBytes))?
                    .to_le_bytes(),
            );
            output.extend(payload);
            Ok(output)
        }
        ArtifactFormat::Ron => ron::ser::to_string_pretty(
            &artifact,
            ron::ser::PrettyConfig::default().struct_names(true),
        )
        .map(|output| output.into_bytes())
        .map_err(|error| CodecError::Malformed(error.to_string())),
    }
}

pub fn decode(
    input: &[u8],
    expected: Option<ArtifactStage>,
    limits: DecodeLimits,
) -> Result<ArtifactEnvelope, CodecError> {
    if input.len() > limits.max_input_bytes {
        return Err(CodecError::Limit(ResourceLimit::InputBytes));
    }
    let budget = Budget::new(limits);
    if !input.starts_with(MAGIC) {
        let workspace = input
            .len()
            .checked_mul(2)
            .ok_or(CodecError::Limit(ResourceLimit::AllocationBytes))?;
        budget
            .reserve_workspace(workspace)
            .map_err(CodecError::Limit)?;
        let options = ron::Options::default().with_recursion_limit(limits.max_depth);
        let mut deserializer = ron::de::Deserializer::from_bytes_with_options(input, &options)
            .map_err(|error| CodecError::Malformed(error.to_string()))?;
        let artifact = ArtifactEnvelope::deserialize(Bounded {
            inner: &mut deserializer,
            budget: &budget,
            depth: 0,
        })
        .map_err(|error| {
            if error == ron::error::Error::ExceededRecursionLimit {
                CodecError::Limit(ResourceLimit::Depth)
            } else {
                budget
                    .failure
                    .get()
                    .map(CodecError::Limit)
                    .unwrap_or_else(|| {
                        CodecError::Malformed(deserializer.span_error(error).to_string())
                    })
            }
        })?;
        deserializer
            .end()
            .map_err(|error| CodecError::Malformed(deserializer.span_error(error).to_string()))?;
        artifact
            .validate(expected.unwrap_or_else(|| artifact.stage()))
            .map_err(CodecError::Validation)?;
        return artifact.canonicalized().map_err(CodecError::Validation);
    }
    if input.len() < HEADER_BYTES || &input[..8] != MAGIC {
        return Err(CodecError::Framing(
            "missing or truncated binary header".into(),
        ));
    }
    let version = u32::from_le_bytes(
        input[8..12]
            .try_into()
            .map_err(|_| CodecError::Framing("schema field".into()))?,
    );
    if version != SCHEMA_VERSION {
        return Err(CodecError::Validation(ArtifactError::SchemaVersion(
            version,
        )));
    }
    let stage = match input[12] {
        1 => ArtifactStage::Analysis,
        2 => ArtifactStage::Elaboration,
        _ => return Err(CodecError::Framing("unknown stage tag".into())),
    };
    let length = u64::from_le_bytes(
        input[13..21]
            .try_into()
            .map_err(|_| CodecError::Framing("length field".into()))?,
    );
    if usize::try_from(length).ok() != Some(input.len() - HEADER_BYTES) {
        return Err(CodecError::Framing("payload length mismatch".into()));
    }
    let mut deserializer = postcard::Deserializer::from_bytes(&input[HEADER_BYTES..]);
    let artifact = ArtifactEnvelope::deserialize(Bounded {
        inner: &mut deserializer,
        budget: &budget,
        depth: 0,
    })
    .map_err(|error| {
        budget
            .failure
            .get()
            .map(CodecError::Limit)
            .unwrap_or_else(|| CodecError::Malformed(error.to_string()))
    })?;
    let remaining = deserializer
        .finalize()
        .map_err(|error| CodecError::Malformed(error.to_string()))?;
    if !remaining.is_empty() {
        return Err(CodecError::Framing("trailing binary data".into()));
    }
    artifact.validate(stage).map_err(CodecError::Validation)?;
    if let Some(expected) = expected {
        artifact
            .validate(expected)
            .map_err(CodecError::Validation)?;
    }
    artifact.canonicalized().map_err(CodecError::Validation)
}

pub fn read_from(
    reader: impl Read,
    expected: Option<ArtifactStage>,
    limits: DecodeLimits,
) -> Result<ArtifactEnvelope, CodecError> {
    let maximum = u64::try_from(limits.max_input_bytes)
        .ok()
        .and_then(|maximum| maximum.checked_add(1))
        .ok_or(CodecError::Limit(ResourceLimit::InputBytes))?;
    let mut bytes = Vec::new();
    reader
        .take(maximum)
        .read_to_end(&mut bytes)
        .map_err(CodecError::Io)?;
    decode(&bytes, expected, limits)
}
