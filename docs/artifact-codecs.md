# Binary and RON Codecs

Plan chunk `008`. The shared model remains the one from
[artifact-model.md](artifact-model.md). Implementation lives in
[../src/artifacts/codec.rs](../src/artifacts/codec.rs) and
[../src/artifacts/bounded.rs](../src/artifacts/bounded.rs).

## Public Boundary

- `ArtifactFormat::{Binary, Ron}` selects output; `Binary` is the default.
- `encode(&ArtifactEnvelope, format)` validates and canonicalizes a copy,
  returning encoded bytes without modifying the input or running any stage.
- `decode(bytes, expected_stage, limits)` detects framing from contents,
  validates the entire envelope and optional expected stage, and returns a
  canonicalized owned artifact. It does not use a filename or read sources.
- `read_from(reader, expected_stage, limits)` reads no more than the input
  limit plus one byte before decoding, and propagates I/O failures. Callers
  own filesystem paths and publication policy; commands/safe writes are `009`,
  catalogs/dependency graph availability are `010`.

Binary starts with `OSVART\0\0` (eight bytes), followed by little-endian `u32`
schema version, `u8` stage (`1` analysis, `2` elaboration), and little-endian
`u64` payload length. The remaining bytes contain one Postcard envelope.
Readers require exact header length, schema support, a known stage, matching
header/payload stage, valid payload compatibility, and complete payload
consumption. Truncated, extra, or mismatched bytes are errors.

RON output starts with named `ArtifactEnvelope(...)`, retaining readable
records, names, IDs, enum variants, source bytes, dependencies, and payload.
It is the same Serde schema, not binary inside a string or a debug-only model.
The checked-in readable synthetic example is
[../tests/fixtures/artifacts/analysis.ron](../tests/fixtures/artifacts/analysis.ron).
RON reads use the typed schema and require complete input consumption.
Unknown fields/revisions, invalid IDs, incompatible versions, and invalid
logical data fail rather than being ignored. RON text is UTF-8; embedded VHDL
source data remains original Latin-1 bytes.

Both output encodings are deterministic for the same canonical model. Binary
to RON to binary and the opposite direction preserve the model and BLAKE3
identity. Formatting whitespace is not identity. No analysis, elaboration,
simulation, original-source lookup, or standard-package body is involved.

## Input Budgets

`DecodeLimits` defaults match the decisions in
[execution-and-artifacts.md](execution-and-artifacts.md):

| Resource | Default |
| --- | --- |
| Encoded input | 64 MiB |
| Entries per collection | 1,000,000 |
| Bytes per string | 1 MiB |
| Cumulative entries | 4,000,000 |
| Charged decoded allocation | 256 MiB |
| Structural depth | 128 |

Both decoders pass through one Serde wrapper. It rejects excessive known
collection lengths, hides allocation hints from model visitors, and charges
each actual entry before decoding it. Empty/zero-sized elements still count.
Unknown lengths are checked incrementally, before constructing the next
element. Struct fields and enum discriminants also count toward the total;
fixed schema field/variant names are not user strings. Fixed-size tuples have
schema-known storage, while dynamic collections incur allocation charges.

Allocation accounting is conservative rather than an exact heap measurement:
the first vector element reserves at least eight slots, subsequent elements
two slots, and strings/byte buffers twice their decoded length. Charges use
checked arithmetic and the actual element type, including inline record
storage. RON additionally reserves twice its bounded encoded input length
before parsing for temporary string-unescaping storage. Budget thresholds may
therefore reject an input whose eventual live heap would be smaller; they do
not promise to accept every file below a final heap measurement. Input-buffer
storage is separately bounded by the encoded-input limit.

String lengths are checked before model string allocation. Postcard borrows
its input string slices; RON temporary unescaping uses the reserved parser
workspace. The shared structural guard also wraps nested options, newtypes,
enums, sequences, and maps. RON independently keeps its parser recursion
limit enabled. Postcard suppresses impossible sequence size hints when the
remaining input is shorter; such a hostile length may fail as malformed
truncation before any collection-limit error, without a huge allocation.

These are resource failures, never successful empty artifacts or truncated
logical values. Later schema expansions must use the shared decode path and
retain the guards. Full mutation/fuzz coverage remains assigned to `121`.

## Errors and Checks

`CodecError` distinguishes framing, malformed encoding, typed resource limits,
model validation, and I/O. `diagnostic()` produces the shared structured
artifact-stage error, with exit status `1` and codes `OSV-ARTIFACT-FORMAT`,
`OSV-ARTIFACT-LIMIT`, `OSV-ARTIFACT-COMPATIBILITY`,
`OSV-ARTIFACT-VALIDATION`, or `OSV-ARTIFACT-IO`. New diagnostic-code variants
are appended, preserving the existing serialized variant ordering.

[../tests/artifact_codecs.rs](../tests/artifact_codecs.rs) exercises both
stages and all conversion directions, deterministic output, every resource
limit, matching collection/string/input boundaries, hostile lengths, every
binary truncation point, trailing data, unknown fields/revisions, wrong schema
and stage tags, bounded readers, I/O errors, and named readable RON. It reuses
the synthetic builder from [../tests/support/artifact.rs](../tests/support/artifact.rs)
with the model tests; no fixture is presented as executable VHDL.

```sh
cargo test --test artifact_codecs --test artifacts --locked
cargo fmt --all
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
```
