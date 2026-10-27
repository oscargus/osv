# Fixture and Golden-Test Helpers

Plan chunk `006`. These helpers are test-only infrastructure. They do not
implement VHDL analysis, elaboration, simulation, or the versioned artifact
schema/codecs owned by chunks `007` and `008`.

## Shared Helpers

[../tests/support/fixture.rs](../tests/support/fixture.rs) provides:

- `Fixture<T>::load(name)`: load RON metadata relative to `tests/fixtures`,
  register the listed source files in order in a `SourceMap`, and retain raw
  source bytes with portable `fixtures/...` display names. Missing files and
  nonportable, absolute, or parent-relative paths fail rather than skip.
- `Metadata<T>::from_ron(text)`: deserialize strict typed metadata, accepting LF
  and CRLF checkouts. Unknown fields, stages, codes, and severities are errors.
- `Fixture<T>::check(&Observation<T>)`: compare the fixture kind, stage,
  independently observed exit status, complete ordered diagnostic list
  (stage/code/severity), typed result, and exact output. Missing, unexpected,
  or duplicate diagnostics fail. Failures identify the fixture and field.
- `round_trips(&value)`: check Postcard and pretty RON round trips, both
  conversion directions, logical equality, complete binary consumption, and
  deterministic re-encoding. It accepts any suitable Serde type, including
  source/diagnostic records and later artifact models. These are trusted
  test-value checks, not production artifact readers, headers, compatibility
  checks, identity fingerprints, or hostile-input limits.
- `normalized_trace(resolution_fs, events)`: convert an ordered iterator of
  `(ticks, delta, phase, event)` records into typed `TraceEvent` records with
  exact integer `time_fs`. Reject zero resolution, conversion overflow, and
  regressing time/delta positions. Phase and event types belong to the caller;
  the helper does not prescribe a scheduler or interpret values. It retains
  every phase, delta, duplicate event, and same-position ordering without
  sorting, rounding, deduplicating, or changing event payloads.

[../tests/support/golden.rs](../tests/support/golden.rs) is shared with the
existing diagnostic snapshot tests. `rendered` uses the real shared renderer,
rejects CR/ANSI output, and trims only trailing line whitespace for the golden
comparison. `expected_output` reads existing RON-string snapshots and accepts
LF/CRLF checkouts. Neither helper normalizes diagnostic codes, values, times,
delta positions, phase ordering, or meaningful messages.

## Fixture Metadata

Each metadata file names an explicit fixture kind:

- `ParserProbe` exercises only the pinned parser in VHDL 2008 mode. Success
  means syntax was accepted, not semantic analysis or executable support.
- `HarnessSelfTest` exercises comparison, serialization, or trace helpers
  using manually constructed records. Such a result is never a successful
  run of the simulator. The artifact/elaboration/trace samples are labelled
  `self-test` and use `synthetic.*` names; they do not define production IR.

For example,
[../tests/fixtures/parser/incomplete-entity.ron](../tests/fixtures/parser/incomplete-entity.ron)
contains:

```ron
(
    kind: ParserProbe,
    sources: ["vhdl/incomplete-entity.vhd"],
    expected: (
        stage: Analysis,
        diagnostics: [(stage: Analysis, code: Syntax, severity: Error)],
        exit_status: 1,
        result: (design_units: 1),
        output: "16..16 expected EntityDeclarationEpilogue",
    ),
)
```

`result` is caller-owned typed data, not a stringly typed result map. Use it
for semantic assertions independently of terminal output. The parser adapter
in [../tests/fixtures.rs](../tests/fixtures.rs) records design-unit count,
checks lossless source reproduction and valid byte spans, and obtains status
from actual structured diagnostics. It does not construct actual results from
expected metadata. Its raw parser error text is also compared exactly.

The tiny
[../tests/fixtures/vhdl/empty-entity.vhd](../tests/fixtures/vhdl/empty-entity.vhd)
and
[../tests/fixtures/vhdl/incomplete-entity.vhd](../tests/fixtures/vhdl/incomplete-entity.vhd)
seed valid and invalid VHDL 2008 syntax. Fixture VHDL files have LF checkouts
via [../.gitattributes](../.gitattributes) so raw byte-offset goldens remain
stable. Source loading never changes the bytes; intentional CRLF/Latin-1
coverage remains in the source/diagnostic tests.

## Adding Coverage

Add a VHDL source and a RON expectation, define or reuse a small typed result,
then pass an independently produced observation to `Fixture::check`. Include
the helper modules with `#[path = "support/fixture.rs"] mod fixture;` and,
when needed, `#[path = "support/golden.rs"] mod golden;`. Existing test
targets choose their own runner; no dependency or new framework is required
for each fixture. Sources may be shared or listed across multiple files.

When real stages land, their adapters must call the actual stage APIs and
introduce an explicit stage-run fixture kind, rather than relabelling a parser
probe or self-test. A missing stage/reference tool must fail explicitly, not
skip or synthesize successful results. Unsupported VHDL remains an explicit
stage-specific diagnostic. No standard-library bodies are loaded by this
harness, and it never runs elaboration or simulation during parsing or format
conversion.

The self-tests demonstrate that wrong expected output/stage/status/results,
missing or unexpected diagnostics, wrong codes/severities/diagnostic stages,
changed trace values/times/deltas/phases/order, malformed metadata, and missing
files all fail. Existing diagnostic snapshots stay unchanged. CI already
runs both integration-test targets on its four platform/architecture runners;
no hosted result for this change is claimed.

```sh
cargo test --test fixtures --locked
cargo test --test diagnostics --locked
cargo fmt --all
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
```
