# Source Management and Diagnostics

Plan chunk `005`. These are shared data and rendering APIs, not a production
VHDL parser adapter, semantic analyzer, artifact format, or simulator.

## Source Records

[../src/source.rs](../src/source.rs) exposes:

- `SourceId(u32)`: an assigned file/source identity, independent of paths and
  display names. IDs are local to a source table and must be retained when
  restoring that table; no global or content-derived identity is claimed.
- `ByteSpan { source, start, end }`: half-open, zero-based original-byte offsets
  using `u32`. `checked_range` rejects reversed and out-of-bounds locations and
  permits empty sources and zero-width EOF locations.
- `SourceFile { id, display_name, encoding, bytes }`: an owned source record.
  `SourceEncoding::Latin1` is explicit; `bytes: None` represents unavailable
  embedded source, not a request to reopen its display name.
- `SourceMap`: monotonic, non-recycled ID allocation through `add`, explicit-ID
  restoration through `insert`, immutable `get`, and ID-sorted `iter`.
  Duplicate IDs and exhausted IDs are errors, never replacements or wrapping.

Registration accepts two records with the same display name as distinct sources.
Restoring an explicit ID advances the allocation high-water mark. Sources larger
than `u32::MAX` bytes are rejected; exhausting the `u32` ID space fails explicitly.
These are source-table limits, not unsupported VHDL constructs.

Constructors and registry insertion normalize display-only separators to `/`,
including restored records. They preserve drive/UNC prefixes and relative path
components. Paths are never opened, resolved, or treated as source identities.
The existing `diagnostics::display_path` contract remains unchanged.

Original Latin-1 bytes, including CRLF and non-ASCII characters, remain intact.
`SourceFile::text` maps each original byte to its Unicode character. Because
this produces exactly one character per original byte, the renderer uses
Ariadne's **character** coordinates, not byte offsets into the expanded UTF-8
display string. This avoids shifted labels and slicing inside UTF-8 characters.

Source records and spans derive Serde support and round-trip through both
Postcard and RON. They are shared data, not a versioned artifact envelope;
fingerprints, artifact-level budgets, and compatibility validation remain owned
by chunks `007`/`008`/`121`.

## Structured Diagnostics

[../src/diagnostics.rs](../src/diagnostics.rs) exposes `Diagnostic` with:

- Typed `DiagnosticCode`, `DiagnosticStage`, and `Severity` fields, plus a
  message. Stages distinguish command-line, analysis, elaboration, simulation,
  and artifact failures without invoking any stage implementation.
- An optional primary `DiagnosticLabel` and ordered related labels, each with a
  byte span and message. Sources are referenced by ID, not path spelling.
- `DiagnosticNote::Text` and `ExpectedActual` records. Expected/actual strings
  preserve descriptive type/bounds information without introducing semantic
  type models from later chunks.
- An ordered outer-to-inner `DiagnosticContext` stack for units, instances, and
  calls, with optional source locations. No recursive rendering logic belongs
  in semantic code.

Codes currently identify syntax/type mismatches and the existing CLI error-code
spellings. Later features add their own typed variants. Diagnostic advice and
warnings are nonfatal; these severities are not VHDL `severity_level` values.

| Diagnostic Result | Exit Status |
| --- | --- |
| No diagnostics, advice, or warnings only | `0` |
| An error in analysis, elaboration, simulation, or artifact handling | `1` |
| A command-line error | `2` |

`Diagnostic::exit_status` implements this policy, and the slice-level
`exit_status` takes the maximum, including `0` for an empty slice. The existing
bootstrap CLI still owns its help/usage/unimplemented behavior and keeps its
public API and status `2` failures. This chunk does not wire future stages into
the CLI or define runtime assertion/stop/finish policy.

## Rendering and Fallback

`diagnostics::render(&Diagnostic, &SourceMap, &mut impl Write)` returns
`io::Result<()>`. Its implementation is isolated in
[../src/diagnostics/render.rs](../src/diagnostics/render.rs). It renders through
Ariadne with ASCII drawing characters, color disabled, ordered labels, and an
in-memory source cache. Rendering never reads a filesystem path or writes
directly to a terminal. Writer errors propagate to the caller.

Missing source records or missing bytes produce explicit `source unavailable`
notes with a retained display name or `<source N>`, byte offsets, and label text.
Invalid spans produce explicit `invalid byte span` notes before reaching
Ariadne. Available related/context labels still render if the primary source is
unavailable. Notes, expected/actual types, and context survive even when no
source section can be rendered. The sourceless fallback writes notes explicitly
because Ariadne otherwise omits them when there are no label groups.

Use the built-in `Advice`, `Warning`, and `Error` report kinds. Ariadne `0.6.0`
custom report kinds can emit ANSI colors despite `with_color(false)`; the
color-free tests cover every supported severity.

## Verification

[../tests/source.rs](../tests/source.rs) checks raw-byte coordinates, stable and
restored IDs, duplicate/exhausted-ID rejection, and binary/RON preservation.
[../tests/diagnostics.rs](../tests/diagnostics.rs) checks actual parser error
spans, multi-file synthetic type failures, Latin-1/CRLF input, unavailable
sources, invalid/EOF spans, portable labels, context, exit statuses, I/O errors,
and identical rendering after binary/RON conversion.

Three expected-output snapshots are stored under
[../tests/snapshots/diagnostics/syntax.ron](../tests/snapshots/diagnostics/syntax.ron),
[../tests/snapshots/diagnostics/type-mismatch.ron](../tests/snapshots/diagnostics/type-mismatch.ron),
and [../tests/snapshots/diagnostics/unavailable.ron](../tests/snapshots/diagnostics/unavailable.ron).
They are readable RON strings, using escapes for non-ASCII text. Tests strip
only trailing line whitespace for snapshot comparison and independently reject
CR/ANSI output. Expected snapshot loading accepts both LF and CRLF checkouts;
this does not normalize the actual diagnostic output. There is no fixture-metadata
or golden-update framework from chunk `006` yet, and no production parser adapter
from chunk `015`.

```sh
cargo test --test source --locked
cargo test --test diagnostics --locked
cargo fmt --all
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
```
