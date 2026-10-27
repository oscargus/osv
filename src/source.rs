use std::{collections::BTreeMap, error::Error, fmt, ops::Range, path::Path};

use serde::{Deserialize, Serialize};

use crate::diagnostics::display_path;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct SourceId(pub u32);

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ByteSpan {
    pub source: SourceId,
    pub start: u32,
    pub end: u32,
}

impl ByteSpan {
    pub fn checked_range(self, source_len: usize) -> Option<Range<usize>> {
        let start = usize::try_from(self.start).ok()?;
        let end = usize::try_from(self.end).ok()?;
        (start <= end && end <= source_len).then_some(start..end)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SourceEncoding {
    Latin1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceFile {
    pub id: SourceId,
    pub display_name: String,
    pub encoding: SourceEncoding,
    pub bytes: Option<Vec<u8>>,
}

impl SourceFile {
    pub fn new(id: SourceId, path: &Path, bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            id,
            display_name: display_path(path),
            encoding: SourceEncoding::Latin1,
            bytes: Some(bytes.into()),
        }
    }

    pub fn unavailable(id: SourceId, path: &Path) -> Self {
        Self {
            id,
            display_name: display_path(path),
            encoding: SourceEncoding::Latin1,
            bytes: None,
        }
    }

    pub fn text(&self) -> Option<String> {
        self.bytes
            .as_ref()
            .map(|bytes| bytes.iter().map(|byte| char::from(*byte)).collect())
    }
}

#[derive(Clone, Debug, Default)]
pub struct SourceMap {
    files: BTreeMap<SourceId, SourceFile>,
    next_id: u64,
}

impl SourceMap {
    pub fn add(&mut self, path: &Path, bytes: impl Into<Vec<u8>>) -> Result<SourceId, SourceError> {
        let id = SourceId(u32::try_from(self.next_id).map_err(|_| SourceError::IdExhausted)?);
        self.insert(SourceFile::new(id, path, bytes))?;
        Ok(id)
    }

    pub fn insert(&mut self, mut source: SourceFile) -> Result<(), SourceError> {
        if self.files.contains_key(&source.id) {
            return Err(SourceError::DuplicateId(source.id));
        }
        if let Some(bytes) = &source.bytes {
            u32::try_from(bytes.len()).map_err(|_| SourceError::SourceTooLarge)?;
        }
        source.display_name = display_path(Path::new(&source.display_name));
        self.next_id = self.next_id.max(u64::from(source.id.0) + 1);
        self.files.insert(source.id, source);
        Ok(())
    }

    pub fn get(&self, id: SourceId) -> Option<&SourceFile> {
        self.files.get(&id)
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = &SourceFile> {
        self.files.values()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceError {
    DuplicateId(SourceId),
    SourceTooLarge,
    IdExhausted,
}

impl fmt::Display for SourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateId(id) => write!(formatter, "source ID {} is already registered", id.0),
            Self::SourceTooLarge => {
                formatter.write_str("source exceeds the 32-bit byte-span limit")
            }
            Self::IdExhausted => formatter.write_str("source ID space is exhausted"),
        }
    }
}

impl Error for SourceError {}
