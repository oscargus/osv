use std::{
    collections::{BTreeMap, btree_map::Entry},
    fmt,
    io::{self, Write},
    ops::Range,
};

use ariadne::{Cache, CharSet, Config, IndexType, Label, Report, ReportKind, Source};

use super::{Diagnostic, DiagnosticLabel, DiagnosticNote, Severity};
use crate::source::{ByteSpan, SourceFile, SourceId, SourceMap};

type RenderedSpan = (SourceId, Range<usize>);

struct RenderCache<'a> {
    sources: &'a SourceMap,
    entries: BTreeMap<SourceId, Source>,
}

impl Cache<SourceId> for RenderCache<'_> {
    type Storage = String;

    fn fetch(&mut self, id: &SourceId) -> Result<&Source, impl fmt::Debug> {
        let source = match self.entries.entry(*id) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let text = self
                    .sources
                    .get(*id)
                    .and_then(SourceFile::text)
                    .ok_or("source unavailable")?;
                entry.insert(Source::from(text))
            }
        };
        Ok::<_, &'static str>(source)
    }

    fn display<'a>(&self, id: &'a SourceId) -> Option<impl fmt::Display + 'a> {
        Some(source_name(self.sources, *id))
    }
}

fn source_name(sources: &SourceMap, id: SourceId) -> String {
    sources.get(id).map_or_else(
        || format!("<source {}>", id.0),
        |source| source.display_name.clone(),
    )
}

fn checked_span(
    label: &DiagnosticLabel,
    sources: &SourceMap,
    notes: &mut Vec<String>,
) -> Option<RenderedSpan> {
    let ByteSpan { source, start, end } = label.span;
    let name = source_name(sources, source);
    let bytes = sources.get(source).and_then(|file| file.bytes.as_deref());
    let Some(bytes) = bytes else {
        notes.push(format!(
            "source unavailable: {name}:bytes {start}..{end}: {}",
            label.message
        ));
        return None;
    };
    let Some(range) = label.span.checked_range(bytes.len()) else {
        notes.push(format!(
            "invalid byte span: {name}:{start}..{end} (source length {} bytes): {}",
            bytes.len(),
            label.message
        ));
        return None;
    };
    Some((source, range))
}

pub fn render(
    diagnostic: &Diagnostic,
    sources: &SourceMap,
    output: &mut impl Write,
) -> io::Result<()> {
    let mut notes = vec![format!("stage: {}", diagnostic.stage)];
    let mut locations = Vec::new();
    for label in diagnostic.primary.iter().chain(&diagnostic.related) {
        if let Some(span) = checked_span(label, sources, &mut notes) {
            locations.push((span, label.message.clone()));
        }
    }
    for note in &diagnostic.notes {
        match note {
            DiagnosticNote::Text(message) => notes.push(message.clone()),
            DiagnosticNote::ExpectedActual { expected, actual } => {
                notes.push(format!("expected type: {expected}"));
                notes.push(format!("actual type: {actual}"));
            }
        }
    }
    for (depth, context) in diagnostic.context.iter().enumerate() {
        let message = format!("context {}: {} {}", depth + 1, context.kind, context.name);
        notes.push(message.clone());
        if let Some(span) = context.span {
            let label = DiagnosticLabel { span, message };
            if let Some(span) = checked_span(&label, sources, &mut notes) {
                locations.push((span, label.message));
            }
        }
    }
    let has_locations = !locations.is_empty();
    let anchor = locations
        .first()
        .map_or((SourceId(0), 0..0), |(span, _)| span.clone());
    let kind = match diagnostic.severity {
        Severity::Advice => ReportKind::Advice,
        Severity::Warning => ReportKind::Warning,
        Severity::Error => ReportKind::Error,
    };
    let mut report = Report::build(kind, anchor)
        .with_code(diagnostic.code)
        .with_message(&diagnostic.message)
        .with_config(
            Config::default()
                .with_color(false)
                .with_char_set(CharSet::Ascii)
                .with_index_type(IndexType::Char),
        )
        .with_labels(
            locations
                .into_iter()
                .enumerate()
                .map(|(order, (span, message))| {
                    Label::new(span)
                        .with_message(message)
                        .with_order(i32::try_from(order).unwrap_or(i32::MAX))
                }),
        );
    if has_locations {
        report.with_notes(&notes);
    }
    report.finish().write(
        RenderCache {
            sources,
            entries: BTreeMap::new(),
        },
        &mut *output,
    )?;
    if !has_locations {
        for note in notes {
            for line in note.lines() {
                writeln!(output, "  = note: {line}")?;
            }
        }
    }
    Ok(())
}
