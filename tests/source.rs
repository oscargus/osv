use std::path::Path;

use osv::source::{ByteSpan, SourceError, SourceFile, SourceId, SourceMap};

#[test]
fn source_byte_spans_match_latin1_characters() {
    let bytes = b"signal caf\xe9 : integer;\n";
    let source = SourceFile::new(SourceId(7), Path::new(r"sources\example.vhd"), bytes);
    let span = ByteSpan {
        source: source.id,
        start: 7,
        end: 11,
    };
    let range = span.checked_range(bytes.len()).unwrap();
    let text = source.text().unwrap();
    assert_eq!(&bytes[range.clone()], b"caf\xe9");
    assert_eq!(
        text.chars()
            .skip(range.start)
            .take(range.len())
            .collect::<String>(),
        "caf\u{e9}"
    );
    assert!(text.len() > bytes.len());
    assert_eq!(source.display_name, "sources/example.vhd");
    assert_eq!(source.bytes.as_deref(), Some(bytes.as_slice()));
    assert_eq!(
        ByteSpan {
            start: 23,
            end: 23,
            ..span
        }
        .checked_range(bytes.len()),
        Some(23..23)
    );
    assert_eq!(
        ByteSpan {
            start: 11,
            end: 7,
            ..span
        }
        .checked_range(bytes.len()),
        None
    );
    assert_eq!(
        ByteSpan { end: 24, ..span }.checked_range(bytes.len()),
        None
    );
}

#[test]
fn source_ids_are_stable_and_not_derived_from_display_paths() {
    let mut sources = SourceMap::default();
    let first = sources.add(Path::new("same.vhd"), b"first").unwrap();
    let second = sources.add(Path::new("same.vhd"), b"second").unwrap();
    assert_eq!(first, SourceId(0));
    assert_eq!(second, SourceId(1));
    assert_eq!(
        sources.insert(SourceFile::new(
            first,
            Path::new("other.vhd"),
            b"replacement"
        )),
        Err(SourceError::DuplicateId(first)),
    );
    assert_eq!(
        sources.get(first).unwrap().bytes.as_deref(),
        Some(b"first".as_slice())
    );
    sources
        .insert(SourceFile::unavailable(
            SourceId(12),
            Path::new(r"old\design.vhd"),
        ))
        .unwrap();
    assert_eq!(
        sources.add(Path::new("new.vhd"), b"new").unwrap(),
        SourceId(13)
    );
    assert_eq!(
        sources.get(SourceId(12)).unwrap().display_name,
        "old/design.vhd"
    );
    assert!(sources.get(SourceId(99)).is_none());
    assert_eq!(
        sources.iter().map(|source| source.id).collect::<Vec<_>>(),
        vec![first, second, SourceId(12), SourceId(13)]
    );
}

#[test]
fn source_id_exhaustion_is_an_explicit_error() {
    let mut sources = SourceMap::default();
    sources
        .insert(SourceFile::unavailable(
            SourceId(u32::MAX),
            Path::new("last.vhd"),
        ))
        .unwrap();
    assert_eq!(
        sources.add(Path::new("overflow.vhd"), b""),
        Err(SourceError::IdExhausted)
    );
    assert!(sources.get(SourceId(u32::MAX)).is_some());
}

#[test]
fn source_records_preserve_ids_and_bytes_in_both_codecs() {
    let mut sources = SourceMap::default();
    sources
        .insert(SourceFile::new(
            SourceId(42),
            Path::new("embedded.vhd"),
            b"-- caf\xe9\r\n",
        ))
        .unwrap();
    sources
        .insert(SourceFile::unavailable(
            SourceId(3),
            Path::new("missing.vhd"),
        ))
        .unwrap();
    let records = sources.iter().cloned().collect::<Vec<_>>();
    let binary = postcard::to_allocvec(&records).unwrap();
    let from_binary: Vec<SourceFile> = postcard::from_bytes(&binary).unwrap();
    let ron = ron::ser::to_string_pretty(&from_binary, ron::ser::PrettyConfig::default()).unwrap();
    let from_ron: Vec<SourceFile> = ron::from_str(&ron).unwrap();
    assert_eq!(from_ron, records);
    assert_eq!(postcard::to_allocvec(&from_ron).unwrap(), binary);
    assert!(ron.contains("Latin1"));
    assert!(ron.contains("embedded.vhd"));
    let mut restored = SourceMap::default();
    for source in from_ron {
        restored.insert(source).unwrap();
    }
    assert_eq!(
        restored.add(Path::new("next.vhd"), b"next").unwrap(),
        SourceId(43)
    );
}
