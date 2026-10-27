# VHDL 2008 Syntax Adapter

Plan chunk `015`. [../src/syntax.rs](../src/syntax.rs) is the production local
boundary around the pinned `vhdl_syntax` revision. It uses only
`parse_with_standard(VHDLStandard::VHDL2008, bytes)`, never the parser's default
or a selectable later language revision. Dependency assumptions remain those
verified in [parser-source-probe.md](parser-source-probe.md); no parser pin,
feature, semantic-engine dependency, or package body changed.

## Public Contract

`syntax::parse(&SourceFile)` consumes the shared source record's original
Latin-1 bytes, without filesystem reads or UTF-8 conversion. It returns
`ParsedSource` only when there are no syntax/policy diagnostics; failed parses
return `Vec<Diagnostic>` rather than exposing a recovered partial tree as
successful input to semantic analysis. Unavailable bytes and oversized sources
have explicit source diagnostic codes. Supplied source IDs are retained.

`ParsedSource` provides source ID, whole-source byte span, lossless `write_to`,
the root node, and design-unit wrappers including their context clauses.
`SyntaxNode` provides kind, checked text span, children, descendants, direct
tokens, and all tokens. Descendant traversal includes the starting node and
delegates to the parser's iterative lossless walker. `SyntaxToken` provides
kind, checked text span, and Latin-1 text mapped to a Unicode display string.
Original bytes remain available from the source table and lossless writer.

Node/token text spans exclude leading trivia; whole-source spans retain every
byte. `span()` returns a checked `ByteSpan` or boxed rich diagnostic on a
parser-location mismatch, never a clamped, guessed, or wrapped location.
The upstream error prose's inclusive-range description is not followed:
actual half-open offsets, zero-width insertion points, Latin-1 bytes, and
CRLF positions are tested at this boundary.

The adapter reexports the pinned `NodeKind` and `TokenKind` for syntax matching
instead of duplicating a second grammar/kind taxonomy. Concrete expression
kinds include `BinaryExpression` and `LiteralExpression`; there is no generic
`NodeKind::Expression` at this revision. Upstream node/token ownership is
private in local views. None of `ParsedSource`, `SyntaxNode`, or `SyntaxToken`
implements Serde. Semantic artifacts must use the independent owned model,
not syntax views, parser arcs, or raw debug text.

## Diagnostics and Explicit Gaps

Parser `SyntaxErr::span` and `err` are mapped directly to shared analysis-stage
errors; rendered error text is never parsed back into structure. Messages use
the parser's structured error-kind display, while OSV supplies source IDs,
checked spans, labels, and its own Ariadne renderer. Missing/invalid locations
retain useful notes instead of panicking. New source/PSL diagnostic variants
are appended, preserving existing variant ordering and status policy.

Coverage `L067` / `lang.psl` is an explicit pinned-parser gap. The adapter walks
the parser's actual token trivia to detect line-comment `-- psl` directives,
including case-insensitive markers and end-of-file comments, and returns
`OSV-PSL-UNSUPPORTED` with source and coverage information. Such assertions
cannot silently become comments/no-ops. It does not hand-roll a lexer or a
PSL grammar. Ordinary `pslike` comments and a string containing `-- psl` are
not treated as directives. Standalone unsupported syntax remains a real
parser error. PSL implementation/scope closure remains `119`.

Known VHDL 2019 mode-view syntax is rejected under the fixed 2008 configuration.
This is a tested revision guard, not a claim that the upstream parser's revision
implementation or every VHDL 2008 feature is complete. Names, type compatibility,
overload/visibility rules, and standard/native declarations remain semantic
work. In particular coverage D04's later `std.textio` interfaces must not be
installed by the native prelude/I/O implementation: a function-name spelling
alone is not a syntax revision rule, and a legal user declaration with the same
spelling must not be rejected by this adapter.

## Verification

[../tests/syntax.rs](../tests/syntax.rs) uses shared fixture metadata and real
VHDL files to parse an entity, architecture, package, expressions, context
clauses, and `process(all)`. It verifies lossless bytes, every node/token span,
syntax errors, Latin-1/CRLF locations, missing sources, real diagnostic
rendering, 2019 rejection, PSL errors, and the explicit syntax/semantic boundary.
The accepted undeclared-type/type-mismatch example deliberately demonstrates
that parsing is not name/type analysis. Fixtures use `ParserProbe`, never a
successful analysis/elaboration/simulation claim. CLI semantic stages remain
unimplemented until their later chunks.

```sh
cargo test --test syntax --locked
cargo fmt --all
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
```
