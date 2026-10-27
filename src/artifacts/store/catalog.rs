use std::{
    collections::BTreeSet,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::artifacts::{
    COMPILER_COMPATIBILITY, CodecError, ContentFingerprint, DecodeLimits, Identifier,
    SCHEMA_VERSION,
    bounded::{Bounded, Budget},
};

use super::{CATALOG_VERSION, LibraryCatalog, StoreError};

const MAGIC: &[u8; 8] = b"OSVCAT\0\0";

pub(super) fn path(root: &Path, library: &Identifier) -> Result<PathBuf, StoreError> {
    let bytes = postcard::to_allocvec(&library.normalized())
        .map_err(|error| StoreError::InvalidCatalog(error.to_string()))?;
    Ok(root
        .join("libraries")
        .join(format!("{}.osvc", ContentFingerprint::of_bytes(&bytes))))
}

pub(super) fn validate(
    catalog: &mut LibraryCatalog,
    library: &Identifier,
) -> Result<(), StoreError> {
    if catalog.version != CATALOG_VERSION
        || catalog.schema_version != SCHEMA_VERSION
        || catalog.compiler_compatibility != COMPILER_COMPATIBILITY
        || catalog.library.normalized() != *library
    {
        return Err(StoreError::InvalidCatalog(
            "incompatible version, compiler, schema, or library".into(),
        ));
    }
    let mut entries = BTreeSet::new();
    for entry in &mut catalog.entries {
        entry.identity = entry.identity.normalized();
        if entry.identity.library != *library
            || !entries.insert((entry.identity.clone(), entry.fingerprint))
        {
            return Err(StoreError::InvalidCatalog(
                "duplicate entry or wrong library".into(),
            ));
        }
    }
    let mut bindings = BTreeSet::new();
    for binding in &mut catalog.bindings {
        binding.identity = binding.identity.normalized();
        if !bindings.insert(binding.identity.clone())
            || !entries.contains(&(binding.identity.clone(), binding.fingerprint))
        {
            return Err(StoreError::InvalidCatalog(
                "duplicate or dangling active binding".into(),
            ));
        }
    }
    catalog.library = library.clone();
    catalog.entries.sort_by(|left, right| {
        (&left.identity, left.fingerprint).cmp(&(&right.identity, right.fingerprint))
    });
    catalog
        .bindings
        .sort_by(|left, right| left.identity.cmp(&right.identity));
    Ok(())
}

pub(super) fn encode(catalog: &LibraryCatalog) -> Result<Vec<u8>, StoreError> {
    let mut bytes = MAGIC.to_vec();
    bytes.extend(
        postcard::to_allocvec(catalog)
            .map_err(|error| StoreError::InvalidCatalog(error.to_string()))?,
    );
    Ok(bytes)
}

pub(super) fn read(
    root: &Path,
    library: &Identifier,
    limits: DecodeLimits,
) -> Result<(LibraryCatalog, Option<ContentFingerprint>), StoreError> {
    let path = path(root, library)?;
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok((
                LibraryCatalog {
                    version: CATALOG_VERSION,
                    schema_version: SCHEMA_VERSION,
                    compiler_compatibility: COMPILER_COMPATIBILITY,
                    library: library.clone(),
                    entries: Vec::new(),
                    bindings: Vec::new(),
                },
                None,
            ));
        }
        Err(error) => return Err(error.into()),
    };
    let maximum = u64::try_from(limits.max_input_bytes)
        .ok()
        .and_then(|maximum| maximum.checked_add(1))
        .ok_or(StoreError::ResourceLimit)?;
    let mut bytes = Vec::new();
    file.take(maximum).read_to_end(&mut bytes)?;
    if bytes.len() > limits.max_input_bytes {
        return Err(StoreError::ResourceLimit);
    }
    if !bytes.starts_with(MAGIC) {
        return Err(StoreError::InvalidCatalog("missing catalog magic".into()));
    }
    let budget = Budget::new(limits);
    let mut deserializer = postcard::Deserializer::from_bytes(&bytes[MAGIC.len()..]);
    let mut catalog = LibraryCatalog::deserialize(Bounded {
        inner: &mut deserializer,
        budget: &budget,
        depth: 0,
    })
    .map_err(|error| {
        budget
            .failure
            .get()
            .map(CodecError::Limit)
            .map(StoreError::Codec)
            .unwrap_or_else(|| StoreError::InvalidCatalog(error.to_string()))
    })?;
    if !deserializer
        .finalize()
        .map_err(|error| StoreError::InvalidCatalog(error.to_string()))?
        .is_empty()
    {
        return Err(StoreError::InvalidCatalog("trailing catalog bytes".into()));
    }
    validate(&mut catalog, library)?;
    Ok((catalog, Some(ContentFingerprint::of_bytes(&bytes))))
}
