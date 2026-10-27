# Execution and Artifact Decisions

Plan chunk `003`, decided and probed on 2026-10-27. This document fixes the
architecture for subsequent chunks; the synthetic experiment in
[../probes/parser/examples/decisions.rs](../probes/parser/examples/decisions.rs)
is not a production artifact schema, VHDL evaluator, or simulator.

## Compiled Execution

Compiled, specialized Rust-kernel dispatch meets the goal in
[../project.md](../project.md). Native machine-code generation from VHDL,
AOT/JIT dependencies, and target-specific executable caches are **not required**.
Do not describe OSV as generating native VHDL machine code. A future backend
would be a separate, explicitly approved change, not a missing prerequisite to
chunk `023`.

The compilation product is an owned, typed executable plan: concrete object and
frame layouts, typed operations, branches, and resumable continuations. Lowering
selects small precompiled Rust kernels for the operand type, width, storage
class, and package policy. A wait saves an explicit continuation location and
runtime frame state; it does not retain a parser node or Rust closure.

Loading validates the entire executable plan and its requirements, resolves each
typed operation/intrinsic key through the versioned registry, checks its operand
layout, and builds an in-memory table of kernel function pointers and parameters.
Unknown keys, bad operand references, and incompatible registry versions fail
before execution. The hot path dispatches these already-bound functions. It may
follow compiled branches and perform required dynamic checks; it must not walk
syntax, resolve names or overloads, or search for a standard package body.

The probe serializes `CheckedAddI64`, reloads it, and exhaustively matches the
typed variant to `i64::checked_add`. Its `BoundOperation` has no Serde
implementation and is reconstructed separately for each load. This demonstrates
the binding mechanism, not the process compiler or event scheduler.

## Stage and Specialization Boundaries

| Stage | Owns | Must Not Do |
| --- | --- | --- |
| Analysis | Declaration/type identity, overload selection when decidable, typed expressions, purity/staticness, source-linked checks and safe folding. | Guess generic actuals, allocate runtime process state, or run file effects. |
| Elaboration | Top/architecture/configuration selection, generics and policy actuals, exact dependencies, hierarchy, concrete bounds/layouts, continuation lowering and final specialization. | Start simulation or leave ordinary name/overload resolution for runtime. |
| Simulation | Fresh mutable state, bound kernels, integer-time scheduling, required dynamic checks and observable effects. | Parse or reanalyze VHDL, repair stale artifacts, or depend on an earlier process's memory. |

Analysis IR uses owned local types and stable typed IDs, never the parser's AST
or arenas. Deferred generic constraints are explicit records with source sites,
not an untyped string expression. Elaboration either discharges each constraint
or emits a deliberate typed runtime check where VHDL requires one. Specialization
keys include declaration/type identity, concrete bounds and direction, signedness,
resolution, package policies, time policy, and intrinsic ABI version. Different
instances must not mutate shared analyzed units or share incompatible layouts.

Constant evaluation and runtime use the same checked kernels where eligible.
Impure calls, signal observations, reports, and required failures are never
removed by folding. Unsupported constructs retain stage-specific diagnostics
and coverage IDs from [coverage.md](coverage.md), not approximate implementations.

## Scalar and Resource Limits

These are OSV implementation choices, not declarations copied wholesale from one
reference simulator. In particular, inventory variants D02 and
`native.std.standard.0010`/`0011` are provenance, not competing builtins.

| Concern | Decision |
| --- | --- |
| `INTEGER` and user integer base types | Signed 64-bit range, `-9223372036854775808` through `9223372036854775807`. `NATURAL` and `POSITIVE` constrain that base range. Subtypes retain their own bounds. |
| Arithmetic | Checked operations and explicit VHDL range/rounding rules, including `MIN / -1`, negation, exponentiation, conversions, and division by zero. No wrapping, saturating, unchecked casts, or Rust-remainder assumptions. Wider/exact intermediates are used when the language operation requires them. |
| Universal numeric literals | Parse/evaluate exact integer or rational magnitudes until contextual conversion; use established big-number primitives when implemented. A literal outside a chosen base range is diagnosed on conversion, not truncated while lexing. |
| Physical values | Signed 64-bit counts of the declared primary unit, with checked unit multipliers and exact conversions. Type and unit identity are separate from storage. |
| `TIME` | Signed 64-bit femtoseconds, including the extra negative endpoint above. Negative time values exist; negative scheduled delays do not. Maximum positive time is `9223372036854775807 fs`, regardless of scheduler resolution. |
| Resolution | Default `1 fs`; supported resolution choices are `1 fs`, `1 ps`, `1 ns`, `1 us`, `1 ms`, and `1 sec`. The chosen policy is fixed in the elaborated artifact. `std.env.resolution_limit` returns it. Nonintegral tick conversion fails explicitly, never rounds silently. |
| Scheduler | Nonnegative checked integer ticks, bounded by `TIME'HIGH / femtoseconds_per_tick`, plus checked `u64` delta positions and explicit phases. Checked wide intermediates handle conversions. No floating-point event times. |
| Scalar `REAL` | IEEE binary64, 53 significant binary bits, finite range `-f64::MAX` through `f64::MAX`, gradual underflow through `2^-1074`, and preserved signed zero. NaN/infinity from scalar operations are domain/overflow errors, not ordinary VHDL real values. |
| Real arithmetic | Round-to-nearest/ties-to-even basic binary64 arithmetic without fast-math reassociation. Apply VHDL conversion/rounding rules explicitly. Transcendentals receive documented domain and error policies in their owning package chunks; no promise of cross-platform bit-identical transcendental results. |
| Vectors | Width is independent of host integer size: packed `u64` through 64 bits, `BigUint` above it, lossless nine-state fallback. Null arrays, direction, declared signedness, and bounds remain explicit. |
| Fixed/floating packages | Exact package bit/format/policy descriptors, not scalar `REAL` or a forced host-width substitute. Format support is subject to declared allocation limits, not silently restricted to 32/64 bits. |
| Artifact input | Initially 64 MiB per artifact in either encoding, 1,000,000 entries per collection, 1 MiB per string, 4,000,000 total entries, 256 MiB charged decoded allocation, and structural depth 128. Check sizes with portable checked arithmetic before allocation. Limits are explicit loader settings and failures are resource diagnostics. |
| Runtime safeguards | Initial defaults: 1,000,000 deltas per timestamp, 10,000,000 executed operations per activation, call depth 1,024. Chunk `059` exposes configurable limits and disabling the optional budgets. Hitting a limit is an unsuccessful resource termination, never queue-empty success. Allocation failures remain errors even with optional budgets disabled. |

The signed 64-bit `INTEGER` choice intentionally exceeds the 32-bit interfaces
in the audited references. Differential fixtures must stay in the shared domain
or explicitly classify this implementation-defined difference; do not normalize
away a mismatch within that domain. Primitive enforcement belongs to `011` and
`031`, not this probe.

## One Logical Artifact Model

Use Serde with **Postcard** (no default features, `alloc` enabled) as the only
production binary codec, and pretty RON. Registry requirements use compatible
version ranges, not exact pins; reviewed resolutions live in the lockfiles.
The current verified releases are Serde `1.0.229`, Postcard `1.1.3`, and RON
`0.12.2`. Upgrades must rerun the shared-model round-trip and rejection checks;
crate version changes do not waive the schema compatibility policy below.
The parser's optional Serde feature stays off.

| Candidate | Probe Result | Decision |
| --- | --- | --- |
| Postcard `1.1.3` | Both synthetic stages and typed variants round-trip; `take_from_bytes` exposes unused input; custom bounded visitors reject excessive sequence lengths before allocating. | Selected: documented stable wire format, architecture-independent varints, small Serde-only configuration, no host-layout coupling. |
| Bincode `2.0.1`, Serde adapter, `standard().with_limit::<4096>()` | Historical experiment: the same envelopes round-tripped with consumed-byte reporting and bounded sequence rejection. | Rejected candidate, not a supported format. Removed from the live probe when updating dependencies: the latest release, `3.0.0`, contains only an unconditional `compile_error!` and no usable codec API. |
| RON `0.12.2` | Same Serde types expose named fields and operation variants; shared count limits and an explicit recursion limit reject hostile inputs. | First-class readable encoding, not an opaque binary field or a separate debug model. |

Postcard does **not** make arbitrary Serde deserialization bounded or validate an
OSV schema by itself. `from_bytes` accepts trailing input in the probe. Production
readers must use `take_from_bytes` and require an empty remainder.

The production envelope/model and codec implementations still belong to `007`
and `008`. Their contract is:

- Both analysis and elaboration carry schema version, stage, compiler
  compatibility epoch, language revision, logical primary/secondary identity,
  owned source/type/symbol tables, exact dependency content fingerprints, and
  native-library/intrinsic version requirements. Stage-specific payloads are
  typed variants, not independent mutable stage/payload strings.
- Analysis stores one primary per artifact, independently versioned secondary
  artifacts linked to their primary, typed declarations/expressions and explicit
  deferred constraints. The catalog belongs to `010`.
- Elaboration stores exact selected primary/secondary/native dependencies,
  concrete hierarchy/types/bounds/object and driver layouts, initial values,
  compiled continuations/operations with source references, and policy/ABI
  requirements. Start with linked artifacts; bundling belongs to `048`.
- Never persist parser nodes, host pointers or function pointers, closures,
  runtime call stacks, event queues, live drivers, RNG state, open handles,
  implementation caches, or hash-map iteration order. Initial state descriptions
  are data; a running simulation is not an artifact snapshot.
- Use fixed-width integer fields and typed enum variants/ID newtypes, not
  `usize`, Rust memory layout, string-parsed IDs, internally/untagged Serde enums,
  `flatten`, or `deserialize_any` in the shared model. Postcard enum ordering and
  field order are schema, so incompatible changes require a version bump.
- A binary file has a fixed header: 8-byte magic `OSVART\0\0`, `u32`
  little-endian schema version, `u8` stage (`1` analysis, `2` elaboration), and
  `u64` little-endian payload length, followed by one Postcard envelope. Header
  and envelope version/stage must agree. RON has the corresponding named
  `ArtifactEnvelope` and fields, not that binary header or encoded bytes.
- Detect binary magic or a valid RON envelope, independent of extensions.
  Validate stage, compatibility and the complete dependency graph before
  binding/execution. A length mismatch, truncation or trailing data is an error.
- Canonical logical identity is BLAKE3-256 of the normalized, versioned logical
  payload encoded canonically with Postcard. Assign stable IDs deterministically,
  sort set-like tables/dependencies by typed identity, preserve semantic
  instruction/element order, and prohibit unordered maps in the persisted
  model. External codec choice, binary framing and RON whitespace are excluded;
  sources, semantic options and version requirements are included. Chunk `007`
  adds the hash dependency and insertion-order tests; no hashing is implemented
  by this experiment.

### Lossless Values

Signed scalars use explicit fixed-width signed fields. Encode scalar real values
as their exact `u64` IEEE bits with readable type metadata, not decimal strings;
check that scalar values are finite separately. RON therefore exposes `real_bits`
and the type table rather than losing negative zero. Package floating values may
carry exceptional values as required by their own format/policy.

Large unsigned magnitudes use canonical little-endian `u32` limbs (zero has an
empty magnitude, no redundant high zero limbs). Width and bounds remain separate.
Signed big intermediates use an explicit sign plus magnitude; negative zero is
not canonical for integers. Nine-state vectors use named state variants in
logical left-to-right element order, including distinct `L`, `H`, and `-`.
Packed vectors have explicit width and bit order: bit zero represents the
rightmost element, independent of index direction. Fixed/floating package values
retain the same exact logic bits plus index/format/policy metadata, including
NaN payloads and signed zeros where the package permits them. No native big-number
or floating type's private memory representation becomes the wire format.

### Bounded Reading and Compatibility

Bound the input before reading it fully (`limit + 1`, including RON), then bound
every collection/string before allocation/copying. Ignore unchecked Serde size
hints for allocation. Charge a shared cumulative element/allocation budget,
including zero-sized elements and nested collections, with checked arithmetic.
RON additionally uses `Options::with_recursion_limit(128)`; neither codec may
deserialize an arbitrary recursive tree without a depth guard. Prefer flat ID
tables. Custom visitors/seeds enforce these limits for both encodings; validating
an already allocated `Vec` is too late. The probe demonstrates the mechanism on
4096 input bytes, 16 fixed-size operations, and RON depth 64, not full production
resource hardening.

Initial compatibility is exact schema, compiler compatibility epoch, native ABI,
and dependency fingerprint matching. There is no implicit migration, best-effort
load, hidden source reanalysis, or name-only intrinsic fallback. The compiler
build version is diagnostic metadata; the compatibility epoch changes whenever
semantic changes invalidate reuse. Pre-release artifacts have no cross-version
compatibility promise. Before a release supporting an older version, record an
explicit migration policy and test both encodings (`121`, `130`). Writes must be
atomic; path/overwrite policy belongs to `009`/`010`.

## Source and Native Identity

Persist original source **bytes**, their content fingerprint, an explicit Latin-1
encoding marker, a portable display label, and half-open byte spans keyed by
stable source IDs. Embed the bytes by default for source-free diagnostics, with
symbol/type names and declaration sites. No parser arena or absolute build path
is needed to recover an error. Future stripped/missing-source artifacts must
still render label, offsets, names/types and a clear source-unavailable note,
not panic or silently read an unrelated file. Rendering Latin-1 into Unicode
must translate byte spans rather than assuming byte offsets remain UTF-8 offsets.

Trusted standard identities combine library/unit identity, the selected 2008
declaration set and fingerprint, declaration/signature IDs, and a versioned
native ABI. `work` inside an IEEE unit resolves to `ieee`. Generic instance
identity includes actuals and policy descriptors. Aliases refer to their resolved
declarations. Inventory row strings are audit/test IDs, not executable keys.
Only trusted exact declarations receive native bindings; a user homonym still
uses its ordinary body. No VHDL standard-package body is an analysis, elaboration,
runtime, conversion, or inspection dependency. Preserve the revision/provenance
and PSL/interface caveats from chunks `001`/`002`.

## Reproduce the Probe

Run from the repository root (the manifest remains isolated):

```sh
cargo run --manifest-path probes/parser/Cargo.toml --locked --example decisions
cargo test --manifest-path probes/parser/Cargo.toml --locked --example decisions decisions_
cargo fmt --manifest-path probes/parser/Cargo.toml
cargo check --manifest-path probes/parser/Cargo.toml --all-targets --locked
cargo clippy --manifest-path probes/parser/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path probes/parser/Cargo.toml --all-targets --locked
```

Seven decision tests cover typed rebinding and deterministic binary/RON conversion
for both stages; checked integer endpoints/overflow and exact real bits; input and
collection boundaries in both formats; huge length-only
prefixes; truncation/trailing data/unknown variants; version/stage rejection; and
RON recursion limits. Synthetic Rust tests are intentional here: there is no
VHDL analyzer or simulator to supply a real fixture yet. Chunk `004` must carry
only Postcard, RON and Serde into the production package, preserve the locked
parser revision, and leave the production schema/runtime to their assigned chunks.
