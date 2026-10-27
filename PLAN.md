# OSV Implementation Plan

This is an execution plan for the simulator described in [project.md](project.md), including the additional requirement that artifacts use binary serialization by default and also support inspectable RON serialization.

Chunks `001`-`009` are complete: parser/source verification, language/native-package inventories, execution/artifact decisions, the Rust library/binary and GitHub Actions bootstrap, shared source management/diagnostics, reusable fixture/golden helpers, owned artifact models/identity, bounded binary/RON codecs, and artifact inspection/conversion commands. Chunk `004` has passing hosted checks on Linux x86-64/ARM64, Windows, and macOS. Production VHDL analysis, elaboration, and simulation are not implemented; checked chunks and their handoffs record verified progress.

## How to Run This Plan

Execute the numbered chunks in order. All earlier chunks are prerequisites unless a chunk explicitly says otherwise. Each chunk should produce one reviewable change, its focused tests, and a short handoff. Do not implement an entire milestone in one agent invocation.

Use this prompt for each invocation, replacing `NNN`:

```text
Read project.md and PLAN.md. Implement chunk NNN only.

Read the handoffs and relevant code from earlier completed chunks. Verify that
the prerequisites exist. Preserve their public contracts unless this chunk
explicitly changes them. Follow repository instructions.

Before editing, identify the owning code path, one testable hypothesis, and
the cheapest check. Make a small change and run that check immediately.
Implement the chunk's Work and Checks without starting later chunks.

Keep unsupported VHDL features explicit; never silently approximate semantics.
Keep analysis, elaboration, and simulation separate. Do not introduce a
dependency on standard-library package bodies. Preserve binary and RON parity.

Run the relevant checks. Mark this chunk [x] only when Done When is satisfied.
Add a short handoff under the chunk: changed interfaces, commands/results,
decisions, and any remaining blockers. Do not mark blocked work complete.
```

For chunks that only establish a decision or inventory, the check is a reproducible probe or a reviewable coverage report rather than a production implementation.

### Chunk Size and Handoffs

- A chunk covers one behavior family and its tests, not a whole compiler subsystem or IEEE package.
- If a chunk proves too large, split it into lettered children, such as `074a` and `074b`, before implementing it. Record their order and acceptance checks under the original chunk. Complete all children before advancing. Do not hide unfinished work in a handoff.
- Package inventories are authoritative: a closure/checkpoint chunk verifies completed work; it is not permission to implement a large collection of missing functions in one change.
- Handoffs belong under the relevant chunk so a fresh agent does not need previous chat history. Durable technical decisions and coverage inventories belong in the repository documentation created by the early chunks.
- Add dependencies only when required by the current chunk. Prefer established Rust crates for parsing, serialization, big integers, diagnostics, and suitable numerical primitives over new implementations of those foundations.
- Do not rewrite the project description or perform unrelated cleanup.

## Requirements and Decisions

### Committed Requirements

1. Rust implementation, using `vhdl_syntax` at an explicit Git commit resolved from the `master` branch of <https://github.com/VHDL-LS/rust_hdl.git>, with that revision recorded in both manifests and lockfiles. Registry dependencies use compatible version ranges rather than exact version pins. VHDL 2008 is the only selectable language revision. Older syntax that remains valid in 2008 is valid input; 2019-only syntax is not.
2. Three user-facing stages: analysis, elaboration, simulation. Simulation consumes elaborated artifacts without reanalyzing VHDL sources.
3. Analysis performs name resolution, type/width checks, safe simplification, and specialization wherever information is available. Generic-dependent decisions are explicitly deferred to elaboration.
4. Each successfully analyzed primary unit has its own serialized artifact. Architectures and package bodies are secondary units and must be retained and linked, not discarded.
5. Elaboration selects a top-level entity and architecture, applies generics and bindings, builds hierarchy, finalizes checks, and produces a simplified serialized executable design.
6. Event-driven execution dispatches specialized small functions. The runtime must not walk parser AST nodes or resolve overloads on each activation.
7. Binary is the final/default artifact encoding. RON is a supported, readable encoding of the same logical schema for both analysis and elaboration artifacts.
8. Native implementations cover all VHDL 2008 `std` packages (`std.standard`, `std.textio`, and `std.env`) and every IEEE 1076-2008 standard-library package, including generic/dependency and compatibility units, without requiring their VHDL package bodies. Both IEEE context declarations are supported as well.
9. Binary-only vectors use efficient integer storage: a small unsigned integer through 64 bits, and `BigUint` or equivalent above 64 bits. Nonbinary logic values have a lossless alternate representation.
10. Errors use `ariadne`, with primary and related spans, expected/actual types and bounds, explanatory notes, and elaboration/runtime context when relevant.
11. Simulation can generate VCD and FST waveform files, independently or together. FST uses `fst_writer` (Cargo package `fst-writer`); both outputs share the same signal registry and simulation observation points.
12. CI uses GitHub Actions for pull requests and pushes. Required jobs cover formatting, compilation, Clippy, tests, native-library inventories, waveform validation, and portable diagnostic output as those features land.

The standard logic package is `ieee.std_logic_1164`; do not invent a `std_1164` alias silently.

### Complete IEEE 2008 Scope

The required IEEE packages are `std_logic_1164`, `numeric_std`, `numeric_std_unsigned`, `numeric_bit`, `numeric_bit_unsigned`, `math_real`, `math_complex`, `fixed_float_types`, `fixed_generic_pkg`, `fixed_pkg`, `float_generic_pkg`, `float_pkg`, and `std_logic_textio`. The required context units are `ieee_std_context` and `ieee_bit_context`. They are analyzed into library `ieee`, including dependencies expressed through the package's current-library `work` alias.

`std_logic_textio` is the 2008 compatibility unit; its historical name does not authorize loading an older body or duplicating I/O declarations already present in `std_logic_1164`. Inventory empty compatibility units as units, not as missing implementations.

Synopsys packages, the older `ieee_proposed` library, VITAL's separate standard, mixed-language/foreign integration, and VHDL-2019-only units are not silently included. The no-package-body requirement applies to every required standard package; user-written package bodies are unaffected.

This plan has 130 ordered chunks. Chunks `001`-`080` retain their IDs; new `081`-`104` cover the additional IEEE families and new `113`-`118` cover waveform output. The original 100-chunk plan's unstarted `081`-`088` are now `105`-`112`, and its `089`-`100` are now `119`-`130`. Inventory ownership and references use the current IDs; no production implementation is introduced by this expansion.

### Architecture to Establish Early

| Concern | Planned Contract |
| --- | --- |
| Module layout | Start with one Cargo package containing a library and the `osv` binary. Separate source/diagnostics, syntax adapter, semantic analysis, artifacts, elaboration, values, intrinsics, lowering, runtime, and CLI through modules. Split crates only for an actual dependency or ownership need. |
| Parsing boundary | Wrap the pinned `vhdl_syntax` API. Do not serialize its internal AST, arena pointers, or unstable node types. Reuse its suitable facilities rather than duplicating them after the parser spike establishes what exists. |
| Analysis IR | Owned, typed, source-linked data with stable IDs, resolved declarations/overloads, and explicit unresolved generic constraints. |
| Elaborated IR | Hierarchy plus concrete types, widths, object/driver layouts, process continuations, intrinsic IDs, and specialized operations. No deferred semantic decisions that simulation cannot check safely. |
| Meaning of compiled | Decided in chunk 003: compile VHDL to typed executable continuations and operation plans that dispatch precompiled Rust kernels. This meets the project goal; native AOT/JIT generation is not required. This is not AST interpretation or a claim of native machine-code generation from VHDL. See [docs/execution-and-artifacts.md](docs/execution-and-artifacts.md). |
| Executable binding | Serialize stable operation/intrinsic IDs and their parameters. Reconstruct function pointers and dispatch tables when loading. Never serialize host pointers, closures, runtime stacks, or hash-map iteration order. |
| Specialization | Specialize known type/shape/width information during analysis; finalize generic-dependent specialization during elaboration. Keep runtime checks where VHDL genuinely requires them. |
| Source information | Persist stable source references and enough source/name/type information for useful diagnostics and RON inspection. Missing original source files must produce a useful diagnostic fallback, not a panic. |
| Native libraries | Resolve exact declarations in trusted/versioned standard-library units. Native replacement is based on declaration identity and signature, not just a function's spelling. User-defined package bodies remain ordinary VHDL. |
| Time | Checked integer simulation ticks with an explicit resolution policy, delta cycles, and VHDL scheduling phases. Never use floating-point event times. |
| Values | Separate VHDL type, bounds, signedness, and resolution semantics from storage optimizations. Preserve all nine `std_ulogic` values and index direction. |
| Waveform output | Observe effective signal changes through a shared registry/sink boundary. Use an established VCD writer and `fst_writer` for FST, with explicit timescales, type/state mappings, hierarchical selection, and reliable finalization. Disabled dumping must not allocate a full trace or affect scheduling. |
| CI | Use GitHub Actions workflows with pinned toolchains/reference tools, least-privilege permissions, and required pull-request/push checks. Keep ordinary Rust/platform jobs separate from reference-dependent jobs; neither missing tools nor optional local skips constitute a CI pass. |
| Performance | Measure dispatch cost, allocation, vector storage, and scheduler behavior. Optimize only against correctness-tested workloads. |

### Serialization Contract

- Use one versioned logical artifact model with Serde support and two codecs: binary and pretty RON. Select one binary codec in chunk 003 and track reviewed dependency resolutions in lockfiles; do not maintain two competing binary formats.
- Both stage artifacts carry schema version, stage kind, compiler compatibility information, logical unit identity, source/type/symbol tables, dependency fingerprints, and native-library/intrinsic version requirements.
- Analysis artifacts contain a primary unit's analyzed data. Store secondary units as referenced, independently versioned artifacts, indexed by their primary unit. A per-library catalog records available versions and bindings.
- Elaboration records exact primary/secondary/native dependencies. Support linked artifacts first; add a self-contained bundle without changing simulation semantics.
- Define content identity from a deterministic, normalized logical payload, independent of whether the external encoding is binary or RON. Pretty printing and format conversion must not change semantic identity.
- Binary readers recognize a magic/header and validate version, stage, sizes, and dependency compatibility. RON contains the corresponding named envelope and readable records, not an encoded binary blob. Do not infer correctness from filename extensions.
- RON must expose meaningful names, enum variants, widths, bounds, literal values, instructions, and references. IDs are accompanied by symbol/type tables; they are not the only explanation of an object.
- Specify exact encodings for large integers, nine-state vectors, real values, and fixed/floating package values. Preserve signed zero and other distinct states wherever the value model permits them. Do not rely on native memory layout or lossy decimal conversion.
- Reject malformed, oversized, stale, wrong-stage, and incompatible artifacts with structured diagnostics. Write atomically. Treat binary and RON readers as equally important input boundaries.
- Maintain round-trip tests, binary-to-RON-to-binary semantic equality, deterministic output tests, and identical simulation results across encodings from the first end-to-end milestone onward.
- Start with explicit rejection of incompatible versions. Add migrations only when a real supported compatibility boundary exists; record that policy before release.

### Proposed CLI

Exact argument spelling can be settled during implementation, but preserve these capabilities:

```sh
osv analyze --library work --out-dir build --artifact-format binary src/design.vhd
osv elaborate work.tb --generic WIDTH=8 --out build/tb.osve --artifact-format binary
osv simulate build/tb.osve --stop-time 1us
osv simulate build/tb.osve --stop-time 1us --vcd build/tb.vcd
osv simulate build/tb.osve --stop-time 1us --fst build/tb.fst
osv simulate build/tb.osve --stop-time 1us --vcd build/tb.vcd --fst build/tb.fst
osv artifact inspect build/tb.osve
osv artifact convert build/tb.osve --to ron --output build/tb.osve.ron
osv artifact convert build/tb.osve.ron --to binary --output build/tb-copy.osve
```

Both `analyze` and `elaborate` accept `--artifact-format binary|ron`; the default is binary. `simulate` accepts either supported encoding. `inspect` prints readable RON without modifying the input. Format conversion does not reanalyze or elaborate a design.

Waveform generation is optional and is separate from artifact serialization: VCD/FST are simulation outputs, not replacements for binary/RON artifacts. Add hierarchical signal/depth filtering, explicit output paths, and actionable output-error diagnostics in chunk `116`.

## Milestones

| Gate | Chunks | Outcome |
| --- | --- | --- |
| A | 001-010 | Verified dependencies, explicit contracts, diagnostics/test foundation, dual-format artifact infrastructure. |
| B | 011-018 | Lossless values, parser adapter, scopes, native prelude, initial typed expressions. |
| C | 019-029 | First actual analysis -> elaboration -> simulation workflow in both formats. |
| D | 030-040 | Differential testing, structured types/expressions/control flow, ordinary subprograms and packages. |
| E | 041-049 | Generic-aware hierarchical elaboration, bindings/configurations, generic packages, linked/bundled designs. |
| F | 050-060 | Waveforms, resolution, waits, attributes, and full scheduling correctness gate. |
| G | 061-104 | Native cores for every IEEE 1076-2008 package, compatibility/context units, and non-I/O conformance gates. Package I/O completes in 108. |
| H | 105-123 | Remaining language/runtime features, complete standard/native I/O, VCD/FST generation, diagnostic/artifact hardening, coverage closure. |
| I | 124-130 | Profile-guided optimization, robustness, documentation, release gates. |

### Verification Rules

- Give each feature positive, negative, and boundary tests appropriate to its semantics. A parser accepting a construct is not evidence that analysis or simulation supports it.
- Run a focused test immediately after the first meaningful edit. Use fixture IDs or a test-name filter so failures identify the current chunk.
- For every chunk, run `cargo fmt`, `cargo check`, and `cargo clippy`, plus relevant tests; use the isolated probe manifest until the production package exists. CI uses a nonmutating formatting check rather than formatting the checkout silently.
- At each milestone run formatting, Clippy, and the complete Rust test suite, using the repository's pinned toolchain. Initial commands are `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace`.
- Use pinned GHDL with VHDL 2008 and matched time/library settings as a differential reference, not as the only oracle. Use authoritative semantics and hand-derived cases for disputed or implementation-dependent behavior. Normalize only irrelevant paths and unspecified ordering, not real mismatches.
- CI must distinguish a missing optional local reference tool from a passed reference test. Required differential jobs fail when their reference tool is unavailable.
- GitHub Actions must run required checks for pull requests and pushes. Pin actions/toolchains/reference utilities appropriately, restrict token permissions, avoid secrets in untrusted pull-request jobs, and test platform-independent diagnostic paths. Add VCD and FST readback/parity jobs when chunks `114`-`118` land.
- Unsupported constructs fail with a stage-specific, source-linked diagnostic and a coverage-matrix ID. They never become no-ops, zero values, interpreted standard-library fallbacks, or silently altered semantics.
- Language coverage and every requested standard-package declaration/signature are tracked explicitly. Early milestones are subsets. Do not advertise complete VHDL 2008 support while required rows remain deferred.

## A. Foundations

### [x] 001 - Verify Parser and Standard Sources

- **Work:** Verify `vhdl_syntax` from the requested upstream `master` branch, its license, resolved revision, supported Rust toolchain, and VHDL 2008 parser API. Run a minimal probe on an entity/architecture, a package, and malformed input. Identify available semantic facilities and usable span information. Locate versioned, appropriately licensed standard-package declarations.
- **Checks:** Record exact dependency coordinates, commands, successful AST/span observations, and one representative parse diagnostic. Verify that the requested dependency is available rather than guessing crate names or APIs.
- **Done When:** A short parser/source note identifies a reproducible integration path and any blocking gaps.
- **Do Not:** Substitute another parser or build a second parser without an explicit decision. Do not copy standard-package bodies into the runtime.

**Handoff (2026-10-27):** Complete. See [docs/parser-source-probe.md](docs/parser-source-probe.md) and [probes/parser/Cargo.toml](probes/parser/Cargo.toml). The lockfile resolves `master` to `dc8c090f7aeb9a95149eadc5e8b48f170382a8d4`; upstream declares Rust `1.95`, and the probe passed on installed Rust `1.98.0`. Eight tests, formatting, and Clippy pass. AST wrapper shapes, Latin-1 input, half-open byte spans, a structured diagnostic, and the syntax/semantic boundary are verified. No production interfaces or package bodies were introduced. There is no blocker to chunk `002`; inventory the parser's revision/PSL limitations and audit declaration completeness/provenance rather than assuming the bundled files alone define full 2008 coverage.

### [x] 002 - Build Language and Native-Package Inventories

- **Work:** Create a VHDL 2008 feature matrix with analysis, elaboration, runtime, diagnostics, and test columns. Enumerate declarations and overload families for all three VHDL 2008 `std` packages, all thirteen IEEE 1076-2008 packages, and both IEEE contexts listed above. Include every declaration-level dependency, generic-package policy parameter, alias, implicit predefined operation, file/text I/O signature, and environment overload, not only explicitly listed functions in a declaration file. Cross-check the IEEE declarations with NVC as well as GHDL.
- **Checks:** Reconcile the list with the pinned standard sources and a VHDL 2008 reference; distinguish abbreviated declarations and later-revision additions. Assign every row a planned chunk, a test identifier, or an explicit scope decision. Record implementation-defined limits separately from unsupported features.
- **Done When:** Every required `std`/IEEE package and context unit has an auditable coverage inventory, and the standard package spelling, revision scope, and body-free bootstrap are unambiguous.
- **Do Not:** Treat one test per package name as complete coverage. Do not silently add VHDL 2019, foreign-language integration, or a GUI to scope.

**Handoff (2026-10-27):** Complete. [docs/coverage.md](docs/coverage.md) records 68 analysis/elaboration/runtime/diagnostic/test rows, implicit operations, dependencies, generic policies, implementation limits, and revision/scope decisions. [docs/native-declarations.md](docs/native-declarations.md) records 1,870 unique audited profiles/unit rows across all three `std` packages, thirteen IEEE packages, and two IEEE contexts, with owner/test IDs and pinned Rust-HDL/GHDL/NVC source links. [probes/parser/examples/inventory.rs](probes/parser/examples/inventory.rs) generates/checks the report using the existing locked parser; it verifies declaration Git blobs and never inventories package bodies. `cargo fmt`, `cargo check --all-targets --locked`, strict `cargo clippy --all-targets --locked -- -D warnings`, and `cargo test --all-targets --locked` passed using the probe manifest (16 tests total); report `--check`, document diagnostics, and the 130-chunk/matrix/owner checks passed. The project/plan now require all IEEE 2008 units, VCD and FST via `fst_writer`, and GitHub Actions, with 130 ordered chunks. These future features and their test IDs are planned, not implemented. No production interfaces, artifact codecs, native runtime behavior, waveform writers, or CI workflows were introduced. There is no blocker to `003`; retain the explicit PSL/parser gap and source-interface differences rather than silently claiming complete language support.

### [x] 003 - Settle Execution and Artifact Decisions

- **Work:** Record decisions for typed IR, compiled execution, integer/time/real limits, source persistence, specialization boundaries, standard-library identity, and schema compatibility. Probe candidate binary codecs and select one with Serde/RON interoperability and bounded decoding. Explicitly decide whether compiled Rust-kernel dispatch meets the project goal or native AOT/JIT is required.
- **Checks:** Demonstrate a typed operation dispatch and a binary/RON envelope round trip in small probes. Identify how function bindings are reconstructed. If native code is required, insert bounded backend tasks before chunk 023 and account for target-specific caches and source mapping.
- **Done When:** No agent needs to guess what compiled means, which binary format to use, or what belongs in a persisted artifact.
- **Do Not:** Call AST interpretation compiled execution or silently defer a required native backend until after the architecture is built.

**Handoff (2026-10-27):** Complete. [docs/execution-and-artifacts.md](docs/execution-and-artifacts.md) fixes typed IR/continuation ownership, checked scalar/time/real limits, source persistence, specialization, trusted native identity, bounded reading, canonical identity, and exact compatibility rejection. Precompiled specialized Rust-kernel dispatch meets the goal; native AOT/JIT is not required. Postcard `1.1.3` is the sole selected binary codec with Serde `1.0.228` and RON `0.11.0`; Bincode `2.0.1` remains a probe-only candidate. [probes/parser/examples/decisions.rs](probes/parser/examples/decisions.rs) demonstrates both-stage binary/RON conversion, load-time function rebinding, deterministic output, overflow/real-bit fidelity, hostile lengths, recursion/truncation/trailing data, and version/stage/operation rejection. The synthetic envelope measures 16 Postcard bytes versus 15 Bincode bytes; size is not the selection criterion. `cargo fmt`, `cargo check --all-targets --locked`, strict `cargo clippy --all-targets --locked -- -D warnings`, and `cargo test --all-targets --locked` pass with the probe manifest (23 tests, including seven new decision tests); the runnable example passes. The parser revision is unchanged. No production schema, codec, value layer, runtime, or standard bodies were introduced. No blocker to `004`; use the chosen codecs and preserve the parser lock. Production bounds/schema/fingerprints remain owned by `007`/`008`/`121`, not implemented by this experiment.

**Dependency Follow-up (2026-10-27):** Registry requirements now use compatible ranges, resolving Postcard `1.1.3`, RON `0.12.2`, and Serde `1.0.229`. The rejected Bincode candidate is retained only as historical comparison evidence, not a dependency: its latest `3.0.0` release intentionally fails compilation. The seven live decision probes continue to verify the selected binary/RON formats and binding contract.

### [x] 004 - Bootstrap Rust and GitHub Actions

- **Work:** Create the minimal `osv` Cargo library/binary, module boundaries, pinned toolchain/MSRV policy, and GitHub Actions workflows for `pull_request` and `push`. Configure read-only default permissions, pinned actions/toolchains, lockfile-aware caches, and Linux plus cross-platform diagnostic checks. Add the verified Git `master` parser dependency from chunk `001` with a deliberately resolved lockfile, Serde, chosen binary codec, RON, and `ariadne`; introduce other dependencies only when needed. Make `osv --help` identify the intended commands without pretending they work yet.
- **Checks:** Build the library and binary; run formatting, `cargo check`, Clippy, and a smoke test in GitHub Actions. Verify workflow syntax/triggers and normalized diagnostic paths on supported runners. Unsupported CLI commands return an explicit nonzero diagnostic.
- **Done When:** A fresh checkout builds reproducibly and required GitHub Actions jobs have working test entry points; future reference and waveform jobs have explicit later owners rather than pretend passing stubs.
- **Do Not:** Scaffold many crates, an application framework, or a full CLI implementation in this chunk.

**Initial Handoff (2026-10-27, hosted blocker resolved below):** Implementation/local verification complete; hosted CI acceptance was pending. See [docs/bootstrap.md](docs/bootstrap.md), [Cargo.toml](Cargo.toml), and [.github/workflows/ci.yml](.github/workflows/ci.yml). One root library/binary reserves the planned module boundaries; `cli::run` provides honest help/version and explicit nonzero unimplemented/usage errors. `diagnostics::display_path` normalizes display-only separators; no source/diagnostic model from `005` was added. Rust/MSRV is pinned to `1.98.0`; root/probe lockfiles retain parser `dc8c090f...`. Only the selected Serde/Postcard/RON and Ariadne dependencies entered production. Root formatting/build/check/strict Clippy and six smoke/portable-diagnostic tests pass; the isolated probe's formatting/check/strict Clippy and 23 tests pass. The 1,870-profile inventory `--check`, checksum-pinned Actionlint `1.7.7`, editor diagnostics, and patch hygiene pass. An exported source tree with empty Cargo cache/target also builds and passes all six production tests using `--locked`. CI defines real push/PR formatting, four-platform/architecture Rust checks, probes/pinned-source inventory, and workflow-lint jobs with read-only permissions, SHA-pinned actions, and lock-aware caches. Differential/waveform/native-runtime gates retain their later owners. At this initial handoff there was no remote or hosted runner access; the acceptance follow-up below records the subsequently published run. No later chunks were started.

**CLI/Dependency Follow-up (2026-10-27):** The CLI now uses typed Clap derives with generated help/version/usage and nested artifact commands while preserving `cli::run`'s writers and explicit unavailable-stage errors. Current compatible ranges resolve Clap `4.6.7`, Ariadne `0.6.0`, Serde `1.0.229`, Postcard `1.1.3`, and RON `0.12.2`; both manifests explicitly select the latest verified upstream `master` commit `dc8c090f7aeb9a95149eadc5e8b48f170382a8d4`. Root smoke coverage reached eight tests; the probe retains 23 tests. Prior fresh-cache results above predate this follow-up. Hosted CI acceptance was still pending at this follow-up; see acceptance below. No later chunk was started.

**Acceptance Handoff (2026-10-27):** Complete. [Hosted CI run 40522628058](https://github.com/oscargus/osv/actions/runs/40522628058) for checkout commit `8a6ac6a259b6376a0d0ae2b23970e0ee5151395b` passed all seven jobs: formatting, workflow syntax, probes/declaration inventory, and Rust build/check/strict Clippy/tests/help on `ubuntu-24.04`, `ubuntu-24.04-arm`, `windows-2022`, and `macos-15`, including diagnostic portability. Both lockfiles are tracked. Local `cargo fmt`, build/check/strict Clippy, all nine root tests, help, and the isolated probe's formatting/check/strict Clippy/23 tests pass. The new focused `bootstrap_artifact_format_options_remain_explicit_errors` test passed immediately after editing; it checks explicit status `2` errors for binary/RON analysis, elaboration, and conversion options. This ninth test is locally verified and postdates the hosted run. No public interfaces, dependencies, parser pins, or stage implementations changed. No remaining chunk `004` blocker; branch-protection administration remains a maintainer responsibility, not a verified repository setting. No later chunks were started.

### [x] 005 - Introduce Source Management and Diagnostics

- **Work:** Add stable source/file IDs, byte spans, a structured diagnostic model, diagnostic codes/severity/related labels/notes, and an `ariadne` renderer. Support labels in multiple files and nested context without embedding rendering logic in semantic code.
- **Checks:** Snapshot syntax and synthetic type-mismatch diagnostics, including non-ASCII source text, multiple files, missing-source fallback, and expected/actual type notes. Check stable exit-status mapping.
- **Done When:** Later stages can report rich diagnostics using shared structured data.
- **Do Not:** Couple semantic errors to terminal output or require the original source path to exist for artifact diagnostics.

**Handoff (2026-10-27):** Complete. See [docs/source-and-diagnostics.md](docs/source-and-diagnostics.md), [src/source.rs](src/source.rs), and [src/diagnostics.rs](src/diagnostics.rs). New contracts: `SourceId`, half-open `ByteSpan`, owned Latin-1 `SourceFile`, and `SourceMap` with stable/importable IDs, checked allocation, duplicate rejection, and deterministic iteration; serializable diagnostic stage/code/severity, primary/related labels, expected/actual notes, and outer-to-inner context frames; `render` writes through Ariadne to an injected writer. Missing sources and invalid spans retain names/offsets/messages without filesystem reads or panics. Latin-1 bytes map to Ariadne character coordinates; source bytes remain unchanged. `exit_status` maps advice/warnings to `0`, stage errors to `1`, and command-line errors to `2`. Existing `display_path` and CLI contracts are unchanged. The first focused byte-span test passed immediately after the initial edit; all three diagnostic snapshots and boundary/codec-parity tests pass, including LF/CRLF snapshot checkouts. Root `cargo fmt --all`, `cargo check --workspace --all-targets --locked`, strict `cargo clippy --workspace --all-targets --locked -- -D warnings`, and `cargo test --workspace --all-targets --locked` pass (26 tests: nine CLI, four source, thirteen diagnostic). The isolated probe's formatting/check/strict Clippy and 23 tests also pass; editor diagnostics and patch hygiene are clean. No dependencies or lockfiles changed. No blockers; no reusable fixture harness, production parser adapter, artifact schema/codecs, or later-stage implementation was added. Existing CI discovers the new test targets; no hosted result for this uncommitted change is claimed.

### [x] 006 - Establish Fixture and Golden-Test Helpers

- **Work:** Add reusable fixtures for analysis errors, artifact round trips, elaboration results, and normalized simulation traces. Define expected stage, diagnostic codes, and exit status in fixture metadata. Seed tiny valid/invalid VHDL 2008 files.
- **Checks:** Demonstrate that intentionally wrong expected output, wrong stage, and missing expected diagnostics fail the harness. Keep snapshots deterministic.
- **Done When:** Subsequent chunks can add a fixture without building a new testing framework.
- **Do Not:** Mark unimplemented simulator behavior as passing or rely solely on text snapshots without semantic assertions.

**Handoff (2026-10-27):** Complete. See [docs/fixtures.md](docs/fixtures.md), [tests/support/fixture.rs](tests/support/fixture.rs), and [tests/fixtures.rs](tests/fixtures.rs). Test-only `Fixture<T>`, `Metadata<T>`, and `Observation<T>` compare kind, stage, exact diagnostic stage/code/severity lists, independent exit status, typed results, and output. Shared golden helpers preserve all three existing diagnostic snapshots. Two LF-pinned VHDL files exercise real VHDL 2008 parser acceptance/rejection; explicitly labelled harness self-tests cover artifact round trips, elaboration results, and normalized traces, not implemented simulator stages. `round_trips` checks Postcard/RON and both conversion directions with logical/deterministic equality; `normalized_trace` uses checked integer femtoseconds and preserves values, phases, deltas, duplicates, and event order. Negative tests reject wrong expectations, missing/extra diagnostics, malformed metadata, missing files, nonportable paths, and trace changes/invalid positions. The first focused syntax snapshot test passed immediately after extraction. Root `cargo fmt --all`, `cargo check --workspace --all-targets --locked`, strict `cargo clippy --workspace --all-targets --locked -- -D warnings`, and `cargo test --workspace --all-targets --locked` pass (41 tests: nine CLI, four source, thirteen diagnostics, fifteen fixture/helper tests). Editor diagnostics, patch hygiene, and LF attributes pass. No dependencies or production interfaces changed; no standard bodies, artifact schema/codecs, analyzer, elaborator, or simulator were introduced. Existing CI discovers the tests; no new hosted result is claimed. No blockers; chunk `007` remains unstarted.

### [x] 007 - Define Owned Artifact Models and Stable IDs

- **Work:** Introduce versioned analysis/elaboration envelopes, unit/type/symbol/source IDs, dependency records, and stage-specific owned payloads. Define canonical ordering, logical content fingerprints, and validation rules. Seed the models with small synthetic artifacts.
- **Checks:** Validate duplicate IDs, dangling references, wrong-stage payloads, and stable identity under insertion-order changes. Verify that no parser arena, host pointer, or function pointer is persisted.
- **Done When:** Both stages have a shared persistence contract that can evolve through deliberate schema changes.
- **Do Not:** Serialize debug output or the parser's private AST as the artifact format.

**Handoff (2026-10-27):** Complete. See [docs/artifact-model.md](docs/artifact-model.md) and [src/artifacts/mod.rs](src/artifacts/mod.rs). Added owned schema/compatibility `1` envelopes with payload-derived stage, VHDL 2008 revision, typed unit/type/symbol IDs (reusing `SourceId`), source fingerprints, exact primary/secondary/dependency/native records, and distinct analysis/elaboration payloads. `validate(expected_stage)`, `canonicalized()`, and `fingerprint()` reject invalid graphs/versions and provide normalized Postcard/BLAKE3-256 logical identity independent of table insertion order and external encoding. Semantic sequences remain ordered; compiler display-version text alone is excluded from identity. BLAKE3 `1.8.7` is resolved through compatible `blake3 = "1"`; parser pins and existing public source/diagnostic contracts are unchanged. The first stage-boundary test passed immediately after the small initial edit. Ten artifact tests cover duplicate/dangling IDs, wrong stages, source fidelity/fallback, secondary links, dependency/native compatibility, type cycles, case and significant order, table permutations, and synthetic elaboration binary/RON equality. Root formatting/check/strict Clippy/all-target tests pass (51 total); Clippy's one local conditional issue was fixed and gates rerun. No parser AST, host/function pointer, live runtime state, production codec/framing, catalog, CLI command, or stage implementation was introduced. No blockers; `008` must add bounded input readers and binary/RON framing without treating these synthetic records as executable designs.

### [x] 008 - Implement Binary and RON Codecs Together

- **Work:** Encode/decode the same artifact models using the chosen binary codec and pretty RON. Add binary magic/version handling and explicit RON envelopes. Implement input limits and consistent error conversion. Keep format detection independent of extensions.
- **Checks:** Round-trip each stage through both encodings and across conversions; check deterministic binary and RON output, wrong versions, truncation, invalid fields, and oversized input. Inspect a RON fixture for named, readable structure rather than an opaque payload.
- **Done When:** Both formats preserve the same logical artifact and semantic identity.
- **Do Not:** Make RON a separate debug schema, a one-way dump, or a wrapper around binary bytes.

**Handoff (2026-10-27):** Complete. See [docs/artifact-codecs.md](docs/artifact-codecs.md), [src/artifacts/codec.rs](src/artifacts/codec.rs), and [src/artifacts/bounded.rs](src/artifacts/bounded.rs). Added default-binary `ArtifactFormat`, canonical `encode`, content-detecting `decode`, bounded `read_from`, explicit `DecodeLimits`/`ResourceLimit`, and `CodecError::diagnostic()` using appended artifact diagnostic codes. Binary validates the specified 21-byte magic/version/stage/length header and complete Postcard consumption; pretty named RON uses the identical model and complete input consumption. A shared seeded Serde layer bounds strings, collections, cumulative entries/allocation, and structural depth before model allocation; RON reserves temporary parser workspace and retains its own recursion guard. Eleven codec tests plus the ten model tests pass: both stages/all conversions, deterministic output/identity, every limit and boundaries, hostile lengths, truncations/trailing data, bad versions/stages/fields, bounded read/I/O errors, and a readable RON fixture. The first bad-header check passed immediately after the initial edit; one associated-lifetime compile error was repaired locally and the same check rerun. Postcard's suppression of impossible sequence hints produces safe malformed-truncation errors, now tested explicitly rather than assumed to be collection-limit errors. Root formatting/check/strict Clippy/all-target tests pass (62 total). No dependencies, logical schema, semantic stages, commands, catalog, or package-body dependency changed. No blockers; `009` should reuse these readers and structured errors for inspection/conversion and implement safe file writes without reanalysis.

### [x] 009 - Add Artifact Inspection and Conversion Commands

- **Work:** Implement `artifact inspect` and `artifact convert`, format selection, explicit output paths, and safe writes using the synthetic artifacts. Share reader/writer code with future stage commands.
- **Checks:** Convert both directions, read files with misleading extensions, preserve content identity, reject incompatible input, and prevent accidental destructive overwrites. Verify that inspection does not modify its input.
- **Done When:** An artifact can be inspected as RON and converted without invoking parsing or elaboration.
- **Do Not:** Add hidden reanalysis, accept malformed data because it is RON, or silently rewrite the input.

**Handoff (2026-10-27):** Complete. See [docs/artifact-commands.md](docs/artifact-commands.md), [src/cli/artifact.rs](src/cli/artifact.rs), and [src/artifacts/files.rs](src/artifacts/files.rs). Typed Clap inspection/conversion arguments now call the existing bounded model codecs; inspection emits named RON read-only, and conversion requires `--to binary|ron --output PATH`. Shared `load` and same-directory `publish_new` expose reusable file boundaries, using `tempfile` `3.27.0` (`tempfile = "3"`) to synchronize complete data and atomically refuse an existing destination, including input aliases. No implicit overwrite/force, reanalysis, source lookup, elaboration, or package body is involved. The first missing-input inspection check passed immediately after the small edit. Six fresh-process tests verify both format directions, misleading extensions, identity/deterministic-binary preservation, read-only inspection, collisions, malformed/incompatible input, argument errors, and temporary cleanup. Root formatting/check/strict Clippy/all-target tests pass (69 total); editor diagnostics are clean. CLI writer/help/version contracts and unavailable semantic-stage errors remain explicit; artifact file failures use structured status `1`, usage failures `2`. No blockers; `010` should build versioned catalogs and atomic replacement over these codec/file helpers rather than reparsing sources or relying on timestamps.

### [ ] 010 - Implement the Library Artifact Store

- **Work:** Store one analyzed artifact per primary unit, secondary artifacts keyed to their primary, and an atomic per-library catalog. Track declaration/body dependencies, content fingerprints, compiler/schema/native-library compatibility, and relevant analysis options. Use synthetic units until real analysis is connected.
- **Checks:** Exercise fresh/reused/stale entries; dependency edits; distinct architecture/package-body versions; interrupted writes; missing references; and invalidation under native-library/schema changes. Distinguish specification changes from body-only changes where semantics permit reuse.
- **Done When:** Stage consumers can resolve an exact compatible artifact graph without reparsing sources.
- **Do Not:** Use timestamps alone for validity or make a single monolithic analysis file the only storage mode.

## B. Values and Frontend Bootstrap

### [ ] 011 - Implement Primitive Values and Simulation Time

- **Work:** Add boolean, bit, enumeration, checked integer, initial real representation, and physical/time primitives following chunk 003's documented limits. Represent time as checked integer ticks and provide common value/range errors.
- **Checks:** Test integer boundaries, overflow, invalid enum positions, time-unit conversion, resolution loss, negative-delay rejection, and both-codec round trips of boundary values.
- **Done When:** Primitive values have explicit semantics and portable serialization, not unchecked host-language behavior.
- **Do Not:** Use floating-point timestamps or assume Rust overflow/casts match VHDL rules.

### [ ] 012 - Implement Array Bounds and Index Mapping

- **Work:** Add constrained/unconstrained range descriptors, ascending/descending directions, logical-to-storage index mapping, null ranges, lengths, and multidimensional shape metadata. Keep array shape separate from element storage.
- **Checks:** Test nonzero lower bounds, `to`/`downto`, null arrays, extreme/overflowing bounds, out-of-range indexing, and shape serialization.
- **Done When:** Vector and general-array operations can share a correct indexing abstraction.
- **Do Not:** Treat all VHDL arrays as zero-based descending bit vectors or reject legal null ranges.

### [ ] 013 - Implement Packed Binary Vectors

- **Work:** Add packed small-vector storage through 64 bits and a `BigUint`-backed representation above it. Define masking, limb/bit order, width-preserving conversion, and signed interpretation without conflating signedness with storage.
- **Checks:** Test widths 0, 1, 31, 32, 63, 64, 65, 128, and a large width; round-trip randomized values; cover leading zeros, negative signed interpretation, and both index directions.
- **Done When:** Binary-only vectors never require a per-bit allocation merely to represent their value, and wide values are not truncated.
- **Do Not:** Use host integer width as the VHDL vector width or store negative signed numbers as an unrelated array format.

### [ ] 014 - Implement Lossless Nine-State Vector Storage

- **Work:** Add scalar logic states `U X 0 1 Z W L H -` and a compact lossless vector fallback. Support promotion from packed storage and safe return to packed storage when every element is exactly `0` or `1`. Keep resolution and numeric interpretation outside the storage layer.
- **Checks:** Property-test transitions at small/big boundaries, all nine states, null vectors, slices, and binary/RON equality. Verify that `L` and `H` do not become `0` and `1` just because storage is optimized.
- **Done When:** Optimized and fallback representations are observably equivalent for the same VHDL value.
- **Do Not:** Collapse unknown/high-impedance/weak values or use a two-state representation for all unsigned operations.

### [ ] 015 - Wrap the Parser Behind a Syntax Adapter

- **Work:** Integrate the verified parser API with source management. Expose the syntax and spans needed by initial analysis through a local adapter; centralize parser-version assumptions and VHDL revision configuration.
- **Checks:** Parse entities, architectures, packages, expressions, and invalid input; preserve source locations; reject known 2019-only constructs under the chosen parser configuration.
- **Done When:** Semantic code consumes a tested local boundary and parser diagnostics use the shared renderer.
- **Do Not:** Implement semantic rules in the adapter or duplicate an existing useful parser facility without cause.

### [ ] 016 - Build Library Scopes and Unit Declarations

- **Work:** Register primary/secondary declarations, parent relationships, lexical scopes, library names, and basic/extended identifier rules. Implement the initial `work`/`std` visibility and declaration lookup needed by the first design.
- **Checks:** Cover duplicate declarations, case rules, missing units, forward dependencies, wrong secondary parents, and stable symbol IDs across artifact reloads.
- **Done When:** Names resolve to declaration identities rather than strings scattered through later passes.
- **Do Not:** Pretend all `use`, overload, context, or package visibility rules are complete before their later chunks.

### [ ] 017 - Bootstrap `std.standard` and Intrinsic Identity

- **Work:** Install trusted builtin declarations for the initial scalar/time types, basic operations, severity, and reporting needs. Create a versioned intrinsic registry keyed by canonical declaration/signature, with metadata for purity, argument/result types, and specialization requirements.
- **Checks:** Resolve initial standard names and operators without package bodies, round-trip intrinsic references, and prove that a user-defined function with the same spelling is not replaced accidentally.
- **Done When:** Analysis, elaboration, and runtime share one body-free builtin identity contract.
- **Do Not:** Substitute functions using only an unqualified name or assume every standard function is pure/foldable.

### [ ] 018 - Type Initial Scalar Expressions

- **Work:** Analyze boolean/integer/time literals and names, selected initial builtin operators, expected types, and locally static expressions. Attach types/spans and perform safe constant folding with explicit not-yet-static results.
- **Checks:** Cover valid expressions, incompatible types, missing declarations, arithmetic/range errors, and a folded versus nonfolded equivalence case. Include diagnostics labeling both operands and their types.
- **Done When:** The initial expression IR is typed and serializable, with no deferred string-based operator lookup.
- **Do Not:** Attempt full overload resolution, all attributes, or speculative folding of impure expressions here.

## C. First End-to-End Simulation

### [ ] 019 - Analyze Minimal Entities and Architectures

- **Work:** Analyze an entity without ports and an associated architecture containing initial scalar constants/signals and processes. Preserve architecture identity, object declarations, default initialization, and source context.
- **Checks:** Cover one legal design, missing/wrong entity associations, duplicate object names, initializer type errors, and unsupported constructs diagnosed at their source.
- **Done When:** A small complete design has typed declarations and a clear boundary for process analysis.
- **Do Not:** Include hierarchy, port maps, generics, or generate statements in this slice.

### [ ] 020 - Analyze a Linear Process Subset

- **Work:** Add scalar signal/variable assignment, `wait for`, indefinite `wait`, `assert`, and `report` to process IR. Establish initial-process execution and variable-versus-signal semantics. Preserve continuation locations and severity information.
- **Checks:** Analyze a process that assigns a boolean signal, waits 1 ns, asserts it, reports success, and waits forever. Reject invalid wait durations and type-incompatible assignments with related spans.
- **Done When:** The first simulation fixture can be analyzed without inserting placeholder operations.
- **Do Not:** Silently lower unimplemented loops, event waits, or subprogram calls to no-ops.

### [ ] 021 - Connect the Analysis Command to Real Artifacts

- **Work:** Implement `analyze` for the supported subset, library/output options, dependency diagnostics, and successful primary/secondary persistence. Expose `--artifact-format binary|ron`, defaulting to binary.
- **Checks:** Analyze multiple units in one file and across files, then resolve them in a fresh process. Compare both formats. Ensure failed analysis does not publish partially valid replacement entries.
- **Done When:** Each successful primary unit and its analyzed secondary units are recoverable from the library catalog.
- **Do Not:** Make file boundaries equal design-unit boundaries or write artifacts for units whose analysis failed.

### [ ] 022 - Elaborate a Single Root Architecture

- **Work:** Select an explicit top entity and unambiguous/default or explicitly requested architecture. Allocate initial scalar objects/drivers and process instances, resolve initial values, and carry the dependency/source/type tables into an elaborated design model.
- **Checks:** Cover missing tops, ambiguous architectures, illegal initialization, isolated repeated elaboration, and deterministic object layouts.
- **Done When:** Elaboration consumes analyzed artifacts rather than parser data and produces a concrete single-instance design.
- **Do Not:** Start simulation during elaboration or invent values for unresolved generics and bounds.

### [ ] 023 - Lower Processes to Compiled Continuations

- **Work:** Compile the supported process subset to typed operations and resumable continuations using the backend decision from chunk 003. Select specialized scalar operations and stable dispatch keys; bind executable functions outside the serialized model. Record source locations for each resumable operation.
- **Checks:** Inspect lowered IR for a wait/resume process, verify exact resumption points, reject invalid operation references, and confirm runtime execution needs no syntax-tree traversal or overload resolution.
- **Done When:** The first design has an executable, typed process plan with no serialized host pointers.
- **Do Not:** Add a general AST interpreter as an unnoticed shortcut or store process state in unserializable closures.

### [ ] 024 - Persist Elaborated Designs and Add the Command

- **Work:** Implement `elaborate` for the supported subset, including top/architecture selection, output path, linked dependencies, and both artifact encodings. Validate the entire elaborated payload before publication.
- **Checks:** Elaborate after restarting the CLI; read binary and RON analysis artifacts; write each elaborated format; reject wrong-stage, stale, or missing dependencies. Compare logical outputs across all format combinations.
- **Done When:** A later process can load a complete executable single-root design from disk.
- **Do Not:** Require an in-memory object left behind by `analyze` or silently reanalyze sources to repair stale input.

### [ ] 025 - Implement the Initial Event Queue

- **Work:** Add checked simulation time advancement, delta-cycle positions, explicit scheduling phases, deterministic legal tie-breaking, and event cancellation tokens. Begin with timed process wakes and scheduled driver updates.
- **Checks:** Test same-time events, zero-delay scheduling, empty queues, cancelled wakes, timestamp overflow, and time boundaries. Check that queue ordering cannot run a process before the updates that should wake it.
- **Done When:** A small synthetic schedule advances predictably through time and delta cycles.
- **Do Not:** Approximate a delta cycle as a positive time delay or assume a single FIFO is the full VHDL scheduler.

### [ ] 026 - Implement Initial Signal Drivers and Updates

- **Work:** Add per-process drivers for the initial unresolved scalar signals, schedule zero-delay assignments, batch driver updates, compute effective values, and distinguish a transaction from an actual value change. Initialize drivers before process execution according to the design model.
- **Checks:** Verify delayed visibility of signal assignments versus immediate variable assignments, unchanged-value transactions, multiple assignments in one process, and unsupported multiple-driver errors.
- **Done When:** Scalar signal updates integrate correctly with the initial time/delta queue.
- **Do Not:** Implement signal assignment as an immediate variable write or allow unresolved conflicts silently.

### [ ] 027 - Execute and Suspend Compiled Processes

- **Work:** Bind typed operations to small execution functions, allocate per-process local state, run initial activations, suspend on timed/indefinite waits, and resume the correct continuation. Wire the initial assertion/report operations into runtime diagnostics.
- **Checks:** Exercise multiple waits, independent process locals, variable updates, signal scheduling, report severity, assertion failure source mapping, and a process that suspends permanently.
- **Done When:** The kernel runs the first real process plan without interpreting parser AST nodes.
- **Do Not:** Add all control flow or procedure suspension in the same change.

### [ ] 028 - Add the Simulation Command and Lifecycle

- **Work:** Implement `simulate` for either elaborated encoding, queue-empty completion, stop-time limits, consistent assertion exit codes, and a structured trace sink usable by tests. Support bounded diagnostic context for runtime failures.
- **Checks:** Run a saved design from a fresh process, check exact stop-time behavior and zero-time work, and verify clean success/failure exits. Reject corrupt and incompatible artifacts before executing them.
- **Done When:** The three stages are usable independently through the CLI for the initial subset.
- **Do Not:** Reparse VHDL at simulation start or equate every informational report with failure.

### [ ] 029 - Gate the First Complete Workflow

- **Work:** Add the smoke design below and exercise all binary/RON stage combinations, inspection, and conversion. Run it in an isolated directory using only required artifacts and embedded diagnostic data, without original sources or standard-package bodies.
- **Checks:** Expect a signal update in a delta cycle at time zero, a successful assertion/report at 1 ns, and queue-empty completion. Compare structured traces and exit codes after cross-format conversion. Run the full milestone checks.
- **Done When:** Analysis -> artifact -> elaboration -> artifact -> simulation works with restart boundaries and no hidden source/package-body dependency.
- **Do Not:** Expand the fixture to unsupported features just to demonstrate a larger design.

```vhdl
entity smoke is
end entity;

architecture sim of smoke is
  signal done : boolean := false;
begin
  process
  begin
    done <= true;
    wait for 1 ns;
    assert done report "signal update missing" severity failure;
    report "smoke passed";
    wait;
  end process;
end architecture;
```

## D. Types, Expressions, and Subprograms

### [ ] 030 - Add Differential Simulation Testing

- **Work:** Extend the fixture harness to run a pinned GHDL configuration with VHDL 2008, matched time resolution and standard-library versions. Compare typed values, reports, timestamps, and observable event ordering through normalized traces. Establish an explicitly required GitHub Actions job that provisions the reference tool.
- **Checks:** Prove that a deliberately changed value/time fails comparison, a missing reference executable is reported correctly, and the smoke fixture agrees. Document how implementation-defined differences are classified.
- **Done When:** Future semantic chunks can add a shared fixture and a reference expectation without bespoke scripts.
- **Do Not:** Normalize away delta-cycle errors or assume all reference behavior supersedes the language standard.

### [ ] 031 - Complete Real and Physical Scalar Operations

- **Work:** Implement the planned scalar real and physical-unit arithmetic, comparisons, conversions, rounding, ranges, and overflow/domain diagnostics. Use native helpers only where they implement the required VHDL semantics.
- **Checks:** Cover integer/real conversion boundaries, rounding cases, user physical units, time arithmetic, extreme values, and artifact fidelity. Differential-test representative expressions.
- **Done When:** Scalar operations have declared limits and correct non-host-default behavior.
- **Do Not:** Introduce `math_real` package coverage implicitly or use floating-point arithmetic for simulation time.

### [ ] 032 - Add Declared Types and Subtype Constraints

- **Work:** Extend semantic types to user enumerations, arrays, records, nested composites, and constrained subtypes. Preserve declaration identity, element/index subtypes, direction, and unresolved constraints. Add storage and default initialization for these values.
- **Checks:** Test distinct-but-similar types, legal/illegal constraints, record field selection, multidimensional arrays, null constraints, defaults, and both-codec round trips.
- **Done When:** Type identity and bounds survive analysis, elaboration, assignment, and serialization.
- **Do Not:** Treat structurally similar declared types as interchangeable or implement access/protected types early.

### [ ] 033 - Implement Aggregates, Slices, and Composite Assignment

- **Work:** Add positional/named aggregates, `others`, concatenation, indexing/slices, and constrained assignment of arrays/records. Define copy/alias behavior at the value boundary and preserve logical index correspondence.
- **Checks:** Cover overlapping slices, opposite directions, null arrays, nested/multidimensional aggregates, missing/duplicate choices, and mismatched lengths with related spans.
- **Done When:** Composite values can be constructed and assigned without losing bounds or partially mutating on a failed check.
- **Do Not:** Use host slice offsets as VHDL indices or silently truncate/extend an assignment.

### [ ] 034 - Complete Contextual Overload Resolution

- **Work:** Resolve overloaded operators/subprograms using expected types, universal literals, visibility, parameter association/defaults, and legal conversions. Retain the selected declaration in IR and expose candidate information for diagnostics.
- **Checks:** Test ambiguity, shadowing, explicit qualification, named arguments, universal numeric literals, incorrect candidates, and generic-dependent unresolved choices.
- **Done When:** Calls bind statically when possible and ambiguous/invalid calls identify the relevant candidates and argument types.
- **Do Not:** Pick the first matching name, invent implicit conversions, or defer ordinary overload search to runtime.

### [ ] 035 - Add Type/Array Attributes and Safe Partial Evaluation

- **Work:** Implement applicable scalar/array attributes such as bounds, length, direction, position/value, image/value, predecessor/successor, and ranges according to the inventory. Extend constant evaluation to typed composites and generic-dependent expressions with explicit deferred results.
- **Checks:** Compare folded/nonfolded results, test null arrays and enum/physical attributes, reject static errors at analysis, and defer only genuinely unknown constraints to elaboration.
- **Done When:** Static checks/specialization are as early as valid, without executing impure code or guessing generic values.
- **Do Not:** Implement event-related signal attributes here or fold expressions whose required context is not yet known.

### [ ] 036 - Compile Structured Sequential Control Flow

- **Work:** Add `if`, `case`, loops, `exit`, `next`, and returns as appropriate to the typed process/subprogram IR. Lower branches and loop state to continuations that preserve local values and source positions.
- **Checks:** Cover nested flow, null loops, boundary loop ranges, early exits, waits within legal process loops, invalid control transfers, and suspensions across branches.
- **Done When:** Structured control flow executes through the compiled plan, with no new AST evaluator.
- **Do Not:** Let an internal execution-budget safeguard alter legitimate simulation semantics silently.

### [ ] 037 - Implement Ordinary Function Calls

- **Work:** Analyze and compile user functions, parameter/result checks, local scopes, recursion/call frames where legal, and purity rules. Use the same intrinsic call boundary for native and VHDL functions while keeping bodies distinct.
- **Checks:** Test nested/overloaded calls, recursion limits/resource diagnostics, composite arguments/results, return constraints, and pure versus impure folding rules.
- **Done When:** User function calls work in expressions and legal static/elaboration contexts.
- **Do Not:** Replace user functions by name or allow waits in a function context where VHDL forbids them.

### [ ] 038 - Implement Procedures and Suspended Calls

- **Work:** Add procedure parameters/classes/modes, aliasing and copy behavior, nested call frames, and legal waits inside procedures called from processes. Resume the entire continuation stack correctly after a wait.
- **Checks:** Cover `in`/`out`/`inout`, signal versus variable parameters, same-object aliasing, waits through nested calls, invalid wait contexts, and a composite output constraint failure.
- **Done When:** Procedure calls preserve VHDL object semantics and suspension behavior across runtime activations.
- **Do Not:** Model every parameter as a plain copied Rust value or lose caller state on suspension.

### [ ] 039 - Analyze User Packages, Bodies, and Contexts

- **Work:** Complete package declaration/body linking, deferred constants, subprogram bodies, imports, context declarations/references, and visibility rules needed by user libraries. Persist the appropriate primary/secondary dependencies and initialization data.
- **Checks:** Analyze files in dependency order and after cache reload; test missing/mismatched bodies, private body declarations, ambiguous imports, dependency cycles, and body-only invalidation.
- **Done When:** Ordinary VHDL packages work without being confused with body-free native standard packages.
- **Do Not:** Interpret the no-standard-body requirement as permission to ignore user-written package bodies.

### [ ] 040 - Close Ordinary Predefined Operation Coverage

- **Work:** Reconcile the `std.standard` inventory for predefined scalar/composite operators, equality/ordering, conversions, enumeration operations, and attribute-backed operations implemented so far. Route them through typed native dispatch and add missing bounded operation families as child chunks if necessary.
- **Checks:** Exercise valid and invalid overloads, range failures, null/composite equality, enum boundaries, constant/runtime agreement, and absence of a standard package body.
- **Done When:** Every ordinary predefined-operation row assigned to this milestone has a native implementation and tests. Run full milestone checks.
- **Do Not:** Mark later access/file/protected functionality complete or use this gate to hide an unbounded implementation backlog.

## E. Hierarchical Elaboration

### [ ] 041 - Apply Entity Generics and Finalize Types

- **Work:** Add named/positional generic association, defaults, typed CLI generic overrides, static evaluation, and generic-dependent subtype/bound resolution. Finalize width/range checks and specialize layouts and operations for each instance.
- **Checks:** Elaborate two instances with different widths, including a width above 64 bits; cover defaults, invalid values/types, null ranges, unresolved constants, and source-linked instantiation diagnostics.
- **Done When:** Generic values affect concrete elaborated types without mutating shared analyzed units.
- **Do Not:** Reparse CLI generic values using an ad hoc type-unsafe string replacement or reuse a layout across incompatible generic settings.

### [ ] 042 - Elaborate Instances and Port Maps

- **Work:** Add direct entity instantiation, formal/actual association, port modes, defaults/open connections, subtype compatibility, and conversion/boundary semantics. Allocate per-instance objects and correctly connect signal/driver identities.
- **Checks:** Test named/positional maps, input/output/inout/buffer behavior as applicable, multiple instances, unconstrained ports, direction changes, legal conversions, and invalid associations.
- **Done When:** A small hierarchical design simulates with independent state and correct port propagation.
- **Do Not:** Flatten every port to an untyped wire or accidentally share instance-local variables/drivers.

### [ ] 043 - Elaborate Generate Statements and Blocks

- **Work:** Add static `for`/`if`/`case` generate constructs and block scopes, instance paths, local declarations, and elaboration-time branch/iteration selection. Retain source-to-instance mapping.
- **Checks:** Cover zero iterations, null ranges, nested generates, different local constants per instance, unreachable branch handling, and diagnostics with the complete instance path.
- **Done When:** The serialized elaborated design contains only the selected instances and executable behavior.
- **Do Not:** Leave generate-condition evaluation to the simulation hot path or discard required declaration checks incorrectly.

### [ ] 044 - Add Components and Default Binding

- **Work:** Analyze component declarations/instantiations and implement standard default binding and compatibility checks. Keep component interfaces distinct from bound entity interfaces until binding is resolved.
- **Checks:** Test cross-library bindings, interface mismatches, missing entities, architecture selection, and a component instance equivalent to direct instantiation.
- **Done When:** Default-bound component hierarchies produce the same execution model as direct entity instances.
- **Do Not:** Implement binding as a global string lookup that ignores libraries, scopes, or interface compatibility.

### [ ] 045 - Implement Explicit Configurations

- **Work:** Add configuration declarations/specifications and explicit entity/architecture binding through hierarchy, including applicable generic/port associations. Preserve configurations as primary-unit artifacts and exact selected secondary dependencies.
- **Checks:** Bind two architectures differently, override nested component binding, reload configuration artifacts, and report conflicting/invalid configurations with related declaration sites.
- **Done When:** Elaboration can reproduce a hierarchy from a top/configuration and exact catalog inputs.
- **Do Not:** Choose an arbitrary architecture when explicit configuration rules require another one.

### [ ] 046 - Implement Generic Subprogram Interfaces

- **Work:** Add the VHDL 2008 generic subprogram/interface-type behavior assigned by the inventory, including association, constraints, overload identity, and specialization keys. Reuse ordinary call/type checking rather than introducing a second call engine.
- **Checks:** Exercise legal instantiations, mismatched type/function actuals, multiple specializations, and serialized dependency identity.
- **Done When:** Generic subprogram instantiations lower to concrete, typed executable operations.
- **Do Not:** Substitute textual macro expansion for semantic generic instantiation.

### [ ] 047 - Implement Generic Packages

- **Work:** Analyze and elaborate generic package declarations/instances, interface-package associations, visible declarations, and instantiation-specific identities. Support the standard generic-package policy machinery needed by fixed/floating packages.
- **Checks:** Instantiate one package with different policies/types, check distinct state/type identity where required, validate illegal associations, and reload artifacts for each instance.
- **Done When:** Generic package instances have deterministic semantic identities and can participate in native-library specialization.
- **Do Not:** Hardcode only the default `fixed_pkg`/`float_pkg` names while ignoring their required generic semantics.

### [ ] 048 - Link and Bundle Elaborated Artifact Graphs

- **Work:** Finalize exact dependency linking for hierarchy, configurations, ordinary/native packages, and specialized operations. Add a self-contained bundle option alongside linked storage, deduplicating shared immutable data and retaining source/type/symbol tables.
- **Checks:** Compare linked/bundled simulation; move bundles to an empty directory; reject missing/stale/cyclic invalid graphs; and convert complete artifacts through RON without changing references or identities.
- **Done When:** Simulation needs only a validated bundle or explicitly resolved compatible artifact closure, not ambient source/library state.
- **Do Not:** Serialize host execution caches or depend on absolute build-machine paths for correctness.

### [ ] 049 - Gate Elaborated Specialization and Diagnostics

- **Work:** Audit deferred analysis checks, generic-specific width/type checks, instance layouts, cache keys, dead/static simplification, and operation selection. Add related-span/instantiation-chain diagnostics for errors discovered only during elaboration.
- **Checks:** Compare two generic specializations and both artifact formats; test stale specialized caches and late bound violations; prove static branches/overloads do not execute in runtime. Run full milestone checks.
- **Done When:** All deferred constraints assigned to this milestone are either discharged or emitted as intentional runtime checks with source context.
- **Do Not:** Remove observable impure calls, reports, or failing required checks while simplifying.

## F. Event Scheduling Semantics

### [ ] 050 - Add Transport Waveforms and Projected Transactions

- **Work:** Extend per-driver scheduling to transport delays, multiple waveform elements, projected output queues, and standard transaction replacement/cancellation rules. Keep cancellation independent from unrelated process wakeups.
- **Checks:** Test successive assignments, equal-time transactions, zero delays, waveform ordering errors, cancelled future updates, and variable-computed delays against reference traces.
- **Done When:** Each driver has a correct projected output waveform rather than a list of uncancellable queue entries.
- **Do Not:** Treat transport as a simple append-only event queue or merge independent drivers.

### [ ] 051 - Add Inertial Delay and Rejection Limits

- **Work:** Implement default/explicit inertial delay, `reject`, pulse rejection, and interaction with existing projected transactions according to VHDL rules. Share waveform data structures with transport without conflating the algorithms.
- **Checks:** Cover pulses below/equal/above rejection limits, successive assignments, mixed delays, invalid rejection values, and same-time edge cases with hand-derived and differential expectations.
- **Done When:** Short-pulse behavior and transaction deletion match the standard for each covered case.
- **Do Not:** Approximate inertial scheduling with debounce logic or blanket cancellation of all pending events.

### [ ] 052 - Resolve Multiple Logic Drivers

- **Work:** Connect native `std_logic` resolution to driver batches, distinguish resolved/unresolved signal subtypes, and update effective values only after the applicable driver updates. Retain active versus changed-value information.
- **Checks:** Cover the complete nine-state resolution table, conflicting drivers, weak/strong/high-impedance combinations, same-delta updates, and unresolved multiple-driver errors.
- **Done When:** Multiple drivers produce correct effective scalar/vector logic values and event notifications.
- **Do Not:** Resolve each driver update independently in a way that exposes spurious intermediate effective values.

### [ ] 053 - Add User and Composite Resolution

- **Work:** Support declared resolution functions and applicable composite resolution semantics. Compile user resolution calls through the ordinary function machinery and enforce their language restrictions.
- **Checks:** Test custom enum resolution, array/record elements where legal, driver ordering independence where required, illegal signatures, and invocation behavior for unchanged drivers.
- **Done When:** Resolution is a type/declaration property with native and user-defined implementations, not a hardcoded special case for one signal name.
- **Do Not:** Assume all resolution functions are associative pairwise tables or execute forbidden impure/waiting behavior.

### [ ] 054 - Implement Guarded Signals and Disconnection

- **Work:** Add guarded assignments/blocks, applicable bus/register signal behavior, driver disconnection, and disconnection specifications from the feature matrix. Integrate changes with resolution and projected waveforms.
- **Checks:** Cover enabling/disabling guards, delayed disconnection, no active drivers, retained versus resolved values, and invalid guarded declarations.
- **Done When:** Guarded/disconnected drivers follow the selected VHDL 2008 rules and reference traces.
- **Do Not:** Model a disconnected driver as an ordinary forced zero or high-impedance value in every type.

### [ ] 055 - Complete Wait Conditions and Sensitivity

- **Work:** Add `wait on`, `wait until`, combined `on`/`until`/`for`, process sensitivity lists, and VHDL 2008 `process(all)`. Compute sensitivity using applicable language rules, deduplicate activations, and cancel stale timeout/event registrations on resumption.
- **Checks:** Test events that do/do not satisfy conditions, equal-time timeout/events, implicit sensitivity through calls, duplicate signals, repeated waits, and illegal waits in sensitized processes.
- **Done When:** Processes resume exactly when specified and at most once per applicable activation cycle.
- **Do Not:** Implement `wait until` as polling or resume a suspended continuation twice from a stale registration.

### [ ] 056 - Complete Simulation Phases and Postponed Processes

- **Work:** Implement the remaining VHDL simulation-cycle ordering and postponed-process rules using the language/reference model. Audit initialization, effective updates, ordinary process execution, implicit activity, and postponed execution boundaries.
- **Checks:** Cover zero-delay feedback, multiple runnable processes, postponed observations, illegal zero-time postponed behavior, initialization events, and race cases with only standard-permitted order normalization.
- **Done When:** Scheduler phase transitions are documented and directly tested, not inferred from insertion order.
- **Do Not:** Treat postponed processes as an arbitrary low-priority queue across time boundaries.

### [ ] 057 - Add Direct Signal Observation Attributes

- **Work:** Implement applicable event/activity/history/driving attributes, including `event`, `active`, `last_event`, `last_active`, `last_value`, `driving`, and `driving_value`. Maintain the minimal correct history per signal/driver.
- **Checks:** Distinguish a same-value transaction from a value change, cover initial state and multiple drivers, and verify elapsed-time/history values across delta cycles.
- **Done When:** Observable attributes follow effective signal and driver semantics rather than approximated queue activity.
- **Do Not:** Reset history at each delta or conflate the current process's driver with the resolved signal.

### [ ] 058 - Add Implicit Signal-Producing Attributes

- **Work:** Implement `delayed`, `stable`, `quiet`, and `transaction` as applicable implicit signals with correct scheduling and initialization. Reuse the kernel's cancellation, phase, and dependency machinery.
- **Checks:** Cover optional time arguments, zero delays, unchanged-value activity, repeated activity before a timeout, attribute sensitivity, and chained attribute use.
- **Done When:** Attribute-generated signals produce correct events and can wake processes in the correct cycle.
- **Do Not:** Compute these attributes only when read; their signal events are observable independently.

### [ ] 059 - Add Explicit Stop/Finish and Runaway Diagnostics

- **Work:** Implement every VHDL 2008 `std.env` signature, including stop/finish overloads and `resolution_limit`, with an explicit status/time-resolution policy. Add configurable delta/step/resource limits, deterministic diagnostic traces, and pending-event summaries. Document default limits and how to disable/change safety limits.
- **Checks:** Exercise every environment overload, `resolution_limit` against the configured tick resolution, finish/stop inside nested calls, assertion termination policy, queue-empty completion, infinite zero-time feedback, and a legitimate long finite run under adjusted limits.
- **Done When:** Users can distinguish requested termination, failure, resource limits, and natural completion.
- **Do Not:** Turn a safety limit into silent successful completion or require a VHDL environment package body.

### [ ] 060 - Gate Scheduling Conformance

- **Work:** Assemble focused regression designs combining waveforms, resolution, attributes, sensitivity, hierarchy, and postponed execution. Compare serialized traces after fresh-load runs of binary and RON artifacts.
- **Checks:** Run the scheduling reference suite, determinism repetitions, linked/bundled variants, and full milestone checks. Document every reference discrepancy rather than blanket-ignoring a fixture.
- **Done When:** All scheduling rows assigned to this milestone have passing direct and integration tests.
- **Do Not:** Begin performance-driven event-queue redesign while known scheduling mismatches remain.

## G. Native IEEE Packages

Every package chunk must use the exact signatures/defaults/policies from chunk 002. Tests must run without the package's VHDL body and cover analysis-time selection, elaboration specialization where applicable, runtime behavior, and both artifact encodings. Declaration dependencies are allowed; executable body fallbacks are not. Early core-package gates explicitly retain I/O rows for chunks `107`-`108`, after access/file support in `105`-`106`; only the final gate in `122` claims complete native-library independence.

### [ ] 061 - Complete Logic Tables and Conversions

- **Work:** Implement the `std_logic_1164` scalar/vector logical operators, conversion/normalization functions, and unresolved/resolved type relationships assigned by the inventory. Reuse the nine-state representation and resolution table.
- **Checks:** Exhaust scalar truth tables; compare randomized vector operations for both directions and small/big/fallback storage; cover null vectors and conversion defaults.
- **Done When:** All logical/conversion overload families in this slice are native and representation-independent.
- **Do Not:** Use Rust boolean logic for unknown states or drop weak/high-impedance distinctions outside standard-defined conversions.

### [ ] 062 - Complete Logic Edge Helpers and Package Coverage

- **Work:** Add edge detection and remaining non-I/O `std_logic_1164` VHDL 2008 declarations from the inventory. Verify exact edge semantics against signal history and normalized logic rules. Keep I/O rows explicitly assigned to `108`; split missing non-I/O families into children if needed.
- **Checks:** Cover transitions among all logic states, unchanged transactions, rising/falling edge calls, applicable reductions/helpers, and every declared overload at least once.
- **Done When:** Every non-I/O `std_logic_1164` signature has native coverage with no body dependency; remaining I/O rows have explicit owners and cannot be advertised as complete before `108` and `122`.
- **Do Not:** Define an edge as any transition to the character `1`, or infer completeness from ordinary `0`/`1` tests alone.

### [ ] 063 - Add `numeric_std` Conversions and Resizing

- **Work:** Implement signed/unsigned conversions, integer conversions, resizing, bit interpretation, result bounds, and standard warning/error behavior. Dispatch pure binary values to integer kernels and nonbinary values to exact logic-aware paths.
- **Checks:** Cover sign extension/truncation rules, overflow, null arrays, widths around 64, large widths, ascending inputs, and weak/unknown bits according to each operation's specified behavior.
- **Done When:** Conversion/resize signatures preserve the standard's values, result ranges, and diagnostics.
- **Do Not:** Apply assignment-style truncation everywhere or assume that preserving storage states means every numeric operation treats them identically.

### [ ] 064 - Add `numeric_std` Addition and Subtraction

- **Work:** Implement signed/unsigned addition, subtraction, unary sign operations, and the scalar/vector overload families assigned by the inventory. Select width-aware packed, big-integer, and nine-state kernels.
- **Checks:** Property-test binary arithmetic against a checked mathematical model and compare standard sizing, overflow behavior, signed boundaries, mixed operand lengths, and unknown propagation against the reference.
- **Done When:** Arithmetic results and bounds match each signature without runtime overload resolution.
- **Do Not:** Reuse host result widths or normalize nonbinary inputs contrary to the package specification.

### [ ] 065 - Add `numeric_std` Multiplicative Arithmetic

- **Work:** Implement multiplication, division, `mod`, and `rem`, with signed/unsigned and scalar/vector overloads from the inventory. Reuse proven big-integer operations while applying VHDL sizing and sign semantics explicitly.
- **Checks:** Test zero divisors, negative operands, differing result widths, large operands, null/unknown inputs, and the distinction between `mod` and `rem`.
- **Done When:** All multiplicative families have correct results, diagnostics, and specialized dispatch.
- **Do Not:** Treat Rust remainder or fixed-width multiplication as a complete substitute for VHDL semantics.

### [ ] 066 - Add `numeric_std` Comparison and Logic Families

- **Work:** Implement comparison, matching/minimum/maximum/reduction/logical helpers where declared by the pinned package, including scalar/vector associations and metavalue diagnostics.
- **Checks:** Exhaust representative small values and state combinations; cover sign extension, different widths/directions, null arrays, and the package's specified fallback results.
- **Done When:** Each comparison/helper overload assigned to this slice is implemented and inventoried.
- **Do Not:** Order nine-state enum discriminants numerically to implement numeric comparisons.

### [ ] 067 - Add `numeric_std` Shifts and Rotations

- **Work:** Implement declared shift/rotate functions/operators and remaining bit-position transformations, preserving operand/result constraints and signed fill behavior. Add width-aware fast paths through the existing dispatch mechanism.
- **Checks:** Cover zero, boundary, larger-than-width, and signed counts where permitted; null vectors; both directions; signed/unsigned distinctions; and nonbinary states.
- **Done When:** All shift/rotation families match the pinned standard definitions rather than host shift behavior.
- **Do Not:** Let oversized host shifts panic, mask counts implicitly, or infer semantics solely from an operator's spelling.

### [ ] 068 - Gate Complete `numeric_std` Coverage

- **Work:** Reconcile every non-I/O `numeric_std` declaration/signature with native implementations, static/runtime eligibility, result-shape rules, and tests. Retain all I/O rows for `108`. Identify whether declaration-level dependencies require additional small implementation children; do not silently substitute another package.
- **Checks:** Run the complete package corpus with no standard bodies, cross-format artifacts, exhaustive small-width binary tests, randomized wide tests, and unknown-state cases.
- **Done When:** Every required non-I/O `numeric_std` signature is covered; I/O rows remain explicit and cannot be claimed complete until `108` and `122`.
- **Do Not:** Mark package completion while missing overloads or body fallbacks remain.

### [ ] 069 - Add `math_real` Constants and Elementary Functions

- **Work:** Implement constants and assigned elementary/rounding/sign/root/exponential/logarithmic families with a documented native numerical backend and precision/domain policy. Preserve declared argument/result constraints and report behavior.
- **Checks:** Compare boundary/domain cases and reference vectors, using exact comparisons for discrete results and justified tolerances for permitted real approximation. Check static/runtime agreement where folding is legal.
- **Done When:** The elementary families satisfy the chosen VHDL implementation limits and test tolerances are documented.
- **Do Not:** Claim bit-exact cross-platform transcendental behavior or ignore domain errors because the host library returns NaN.

### [ ] 070 - Add `math_real` Trigonometric Families

- **Work:** Implement trigonometric, inverse, hyperbolic, and multiargument families as listed in the pinned package. Validate backend suitability and special cases rather than forwarding blindly.
- **Checks:** Cover quadrants, branch/domain boundaries, extrema, very small/large inputs, identities with justified tolerances, and expected diagnostics.
- **Done When:** Every function in these families has explicit semantics, native dispatch, and reference cases.
- **Do Not:** Reuse a host function whose domain, argument order, or error behavior differs without adaptation.

### [ ] 071 - Add `math_real` Random Generation and Close Coverage

- **Work:** Implement `uniform` with explicit mutable seed semantics and the standard's required sequence/range behavior. Reconcile all remaining package declarations through bounded children before closing the inventory.
- **Checks:** Compare known seed/result sequences, invalid seeds, independent call sites, repeatability, and forbidden constant folding. Run the package suite without its body.
- **Done When:** The complete requested `math_real` interface is native and stateful operations are not mistaken for pure arithmetic.
- **Do Not:** Substitute an arbitrary Rust RNG or seed from wall-clock time.

### [ ] 072 - Establish Fixed/Floating Policy Types

- **Work:** Implement required `fixed_float_types` declarations and native policy metadata for rounding, overflow, guard bits, denormal handling, and error modes as actually declared. Connect generic package instances and dependency fingerprints to those policies.
- **Checks:** Instantiate distinct policies, verify cache/artifact identities differ, reject illegal policy associations, and prove required declaration dependencies load without executable package bodies.
- **Done When:** Fixed/floating operations can receive complete, explicit policy/format descriptors.
- **Do Not:** Hardcode only one default policy or silently replace all package formats with host floating-point types.

### [ ] 073 - Add Fixed-Point Representation and Conversions

- **Work:** Represent `ufixed`/`sfixed` with their left/right index semantics, signedness, exact underlying logic bits, and binary fast paths. Implement conversion, resize, rounding, and saturation/wrap families assigned by the inventory.
- **Checks:** Cover negative fractional indices, arbitrary integer/fractional widths, rounding ties, overflow policies, unknown bits, null/illegal constraints, and both-codec values.
- **Done When:** Fixed-point values and conversions preserve precision and policy without routing exact arithmetic through `f64`.
- **Do Not:** Treat fixed-point bounds as a simple zero-based vector length or discard nonbinary bits.

### [ ] 074 - Add Fixed-Point Additive and Multiplicative Arithmetic

- **Work:** Implement the assigned add/subtract/negate/absolute/multiply overload families using proven integer primitives and explicit binary-point/result-range rules. Specialize known formats/policies during elaboration.
- **Checks:** Compare exact rational/integer models, negative operands, fractional carries, result sizing, mixed formats, policy boundaries, and logic-aware fallback behavior.
- **Done When:** These arithmetic families are native and exact up to the required rounding/overflow policy.
- **Do Not:** Combine incompatible generic policies or use an approximate host-real computation as the semantic implementation.

### [ ] 075 - Add Fixed-Point Division and Remainder Families

- **Work:** Implement division, reciprocal, remainder/modulo, and other assigned quotient-related functions with exact intermediate arithmetic and required guard/rounding behavior.
- **Checks:** Cover zero divisors, signs, recurring fractions, guard-bit effects, extreme formats, overflow, and invalid/unknown inputs against the reference and exact models.
- **Done When:** Quotient-related families have tested result bounds, policies, and diagnostics.
- **Do Not:** Depend on accidental host integer division rounding or silently lose guard information.

### [ ] 076 - Complete Fixed-Point Helpers and Coverage

- **Work:** Implement the separately inventoried comparison, shift, logic, formatting, and remaining helper families in bounded children as needed. Reconcile default `fixed_pkg` and its necessary generic/declaration dependencies.
- **Checks:** Cover each remaining signature, formatting round trips where specified, policy combinations, both representations, and a small fixed-point design. Run without standard bodies.
- **Done When:** Every required non-I/O fixed-package signature and policy path is covered; I/O rows remain assigned to `108`, and the gate lists no hidden fallback implementation.
- **Do Not:** Assume arithmetic coverage alone means the package interface is complete.

### [ ] 077 - Add Floating-Point Formats and Conversions

- **Work:** Model the package's sign/exponent/fraction formats, logic-valued bits, signed zeros, infinities, NaNs, denormals, and policy descriptors. Implement conversion/classification families using suitable proven exact/soft-float primitives where available.
- **Checks:** Cover standard and non-host widths, conversion rounding, subnormals, exceptional encodings/payloads where relevant, unknown bits, and binary/RON fidelity.
- **Done When:** Package values are not limited to host `f32`/`f64` and remain lossless in storage/artifacts.
- **Do Not:** Assume the package's floating values are ordinary Rust floats or erase exceptional/nonbinary states.

### [ ] 078 - Add Floating-Point Addition and Subtraction

- **Work:** Implement add/subtract and assigned unary families with alignment, normalization, rounding, overflow, underflow, exceptional-value rules, and generic policies.
- **Checks:** Cover cancellation, exponent gaps, rounding ties, signed zeros, infinities/NaNs, denormals, arbitrary supported formats, and nonbinary inputs using trusted vectors/reference results.
- **Done When:** Additive families satisfy the package's format/policy semantics and native dispatch contract.
- **Do Not:** Forward every format through host `f64` or accept double-rounding errors silently.

### [ ] 079 - Add Floating-Point Multiply/Divide/Root Families

- **Work:** Implement multiplication, division, square root, and fused/related families actually declared in the selected package, using the shared format and rounding engine. Split independent heavy families into child chunks.
- **Checks:** Cover exact/inexact results, zero division, exceptional combinations, underflow/overflow, guard behavior, and fused versus separate-operation results where applicable.
- **Done When:** Each assigned family has trusted reference vectors and correct native semantics across required formats.
- **Do Not:** Invent undeclared APIs or implement fused operations as separately rounded operations when the package requires otherwise.

### [ ] 080 - Complete Floating Helpers and Initial IEEE Cores

- **Work:** Finish the inventoried comparison, remainder, decomposition/scaling, formatting, and remaining non-I/O floating helpers through bounded children. Audit the logic/numeric/real/fixed/floating cores implemented so far, their dependencies, policies, and body-free linking. The additional IEEE package families begin at `081`; all package I/O remains assigned to `108`.
- **Checks:** Run per-signature tests, mixed-package designs, both artifact encodings, and full milestone checks with all requested standard package bodies unavailable.
- **Done When:** The non-I/O families assigned through `080` have no body dependency or unimplemented native operation. This is not a gate for the entire IEEE library.
- **Do Not:** Hide remaining signatures behind a broad unsupported-package diagnostic or a reference simulator subprocess.

### [ ] 081 - Add Complex Types and Constants

- **Work:** Install `math_complex` record/subtype declarations, real/polar forms, constants, and the dependency on `math_real`. Bind exact declaration identities without a package body.
- **Checks:** Verify field types, subtype bounds, constants, current-library dependency binding, and binary/RON preservation.
- **Done When:** Native complex values have the declared shape and constraints; no arithmetic is approximated by a placeholder.
- **Do Not:** Replace the VHDL records with an opaque host type or confuse polar bounds with implementation limits.

### [ ] 082 - Add Complex Conversion and Observation Helpers

- **Work:** Implement `cmplx`, Cartesian/polar conversions, principal-value normalization, magnitude, argument, conjugation, and declared equality families.
- **Checks:** Cover zero magnitude, principal-angle boundaries, negative components, both representations, and every overload against reference vectors.
- **Done When:** Each helper has native dispatch, correct domain behavior, and source-linked errors.
- **Do Not:** Assume host complex-library branch conventions match the VHDL package automatically.

### [ ] 083 - Add Complex Additive Arithmetic

- **Work:** Implement declared unary/binary addition and subtraction with real/complex and Cartesian/polar associations. Reuse typed real kernels and proven numerical primitives.
- **Checks:** Exercise every overload, cancellation, signs, mixed real operands, polar normalization, and static/runtime agreement within justified tolerances.
- **Done When:** Additive result forms, constraints, and diagnostic policies match the selected 2008 signatures.
- **Do Not:** Invent mixed-representation overloads not present in the declaration inventory.

### [ ] 084 - Add Complex Multiplication and Division

- **Work:** Implement declared multiply/divide families with their scalar/complex and polar variants, adapting numerical primitives for required normalization and domain rules.
- **Checks:** Cover zero denominators, magnitude extremes, signs/quadrants, declared result types, and comparison with reference results.
- **Done When:** Multiplicative families have native, tested behavior without avoidable intermediate overflow.
- **Do Not:** Rely on unchecked host division or silently convert a required error into a finite value.

### [ ] 085 - Add Complex Elementary Functions

- **Work:** Implement declared square-root, exponential, and logarithmic families, including explicit-base overloads. Document branch, domain, and approximation policies.
- **Checks:** Test branch boundaries, roots of negative-real inputs, invalid bases/zero arguments, extreme values, and Cartesian/polar agreement.
- **Done When:** Every inventoried elementary signature has trusted vectors and native dispatch.
- **Do Not:** Add an undeclared complex power operator or weaken branch/domain requirements to match a host API.

### [ ] 086 - Add Complex Trigonometric Families

- **Work:** Implement the declared sine/cosine and hyperbolic families for both package representations using a documented suitable numerical backend.
- **Checks:** Cover real-axis identities, imaginary components, principal-angle boundaries, large inputs, and justified approximation tolerances.
- **Done When:** Every declared trigonometric overload has static/runtime and reference tests.
- **Do Not:** Promise bit-identical transcendental results across platforms without evidence.

### [ ] 087 - Gate Complete Complex Package Coverage

- **Work:** Reconcile every `math_complex` declaration, constant, overload, dependency, and native registration with the inventory. Add bounded children for any genuine remaining family.
- **Checks:** Run all per-signature tests, mixed `math_real`/complex designs, both artifact formats, and source/body-free deployment.
- **Done When:** The complete `math_complex` surface executes natively and every row has a passing test.
- **Do Not:** Count a package name or a single successful complex calculation as full coverage.

### [ ] 088 - Add Binary Numeric Types and Conversions

- **Work:** Install `numeric_bit` signed/unsigned bit-array types and implement conversions, resizing, and result-bound rules. Reuse packed integer and big-integer storage without logic-state machinery.
- **Checks:** Cover signs, both index directions, null arrays, widths around 64, wide values, conversion failures, and artifact fidelity.
- **Done When:** Native bit-numeric conversion/resize signatures preserve their own declared types and widths.
- **Do Not:** Alias `numeric_bit` type identities to `numeric_std` types merely because binary arithmetic can share kernels.

### [ ] 089 - Add Binary Numeric Additive Arithmetic

- **Work:** Implement `numeric_bit` add/subtract, unary signs, and absolute-value overloads using shared checked kernels with package-specific sizing adapters.
- **Checks:** Exhaust small widths; test differing operand lengths, carry, signed boundaries, scalar associations, and constant/runtime equivalence.
- **Done When:** Every additive signature has correct results, bounds, and native dispatch.
- **Do Not:** Let kernel reuse impose the wrong declaration identity or host-integer result width.

### [ ] 090 - Add Binary Numeric Multiplicative Arithmetic

- **Work:** Implement multiplication, division, `mod`, and `rem` for the inventoried signed/unsigned and scalar/vector combinations.
- **Checks:** Cover zero divisors, negative signs, result widths, wide values, and mathematical versus reference results.
- **Done When:** All multiplicative `numeric_bit` overloads preserve VHDL sizing and remainder semantics.
- **Do Not:** Duplicate an arithmetic engine unnecessarily or equate `mod` and `rem`.

### [ ] 091 - Add Binary Numeric Comparisons and Helpers

- **Work:** Implement declared comparisons/matching, logical/reduction, minimum/maximum, search, and edge-helper families, splitting independent helpers into children if needed.
- **Checks:** Cover every overload, unequal lengths, null arrays, signed extension, bit-signal history, and both index directions.
- **Done When:** All assigned non-I/O helpers have exact bit-domain semantics and typed native bindings.
- **Do Not:** Manufacture nine-state behavior for `bit` or infer an edge from a value without signal history.

### [ ] 092 - Add Binary Numeric Shifts and Rotations

- **Work:** Implement declared shift/rotate functions and operators with signed fill, result-shape, and count semantics using existing kernel utilities.
- **Checks:** Test zero/boundary/oversized and negative counts where permitted, null arrays, signs, and wide widths.
- **Done When:** Every shift/rotation signature matches the bit-numeric package rather than host shift behavior.
- **Do Not:** Mask counts implicitly or let oversized shifts panic.

### [ ] 093 - Gate Binary Numeric Core Coverage

- **Work:** Reconcile `numeric_bit` declarations, non-I/O signatures, conversions, and specialized bindings. Keep all text-I/O rows explicitly assigned to `108`.
- **Checks:** Run exhaustive small-width and randomized wide tests, reference designs, both encodings, and body-free linking.
- **Done When:** Every non-I/O row is tested, and I/O gaps remain visible until `108` and `122`.
- **Do Not:** Advertise full package coverage while its declared I/O overloads are pending.

### [ ] 094 - Add Bit-Unsigned Interfaces and Conversions

- **Work:** Install `numeric_bit_unsigned` declarations and implement the bit-vector/integer conversion and resize adapters over the completed `numeric_bit` kernels.
- **Checks:** Validate exact overload/default selection, natural-value bounds, result direction/length, null arrays, and wide conversions.
- **Done When:** The wrapper package has its own declaration identities and tested conversion adapters.
- **Do Not:** Replace all bit-vector operations globally; visibility and overload rules still select the package operations.

### [ ] 095 - Add Bit-Unsigned Arithmetic Adapters

- **Work:** Implement the declared arithmetic overloads by adapting bit-vector operands/results to native `numeric_bit` operations without copying an entire package body.
- **Checks:** Cover all operand combinations, result sizing, overflow/truncation rules, zero divisors, and widths on either side of 64.
- **Done When:** Each arithmetic wrapper preserves the declared result type and behavior while sharing proven kernels.
- **Do Not:** Add implicit conversions at unrelated call sites or reuse a result width without checking the wrapper signature.

### [ ] 096 - Add Bit-Unsigned Comparison and Shift Adapters

- **Work:** Implement declared matching/comparison, minimum/maximum, search, shift, and rotate helpers over `numeric_bit`, splitting families into children if necessary.
- **Checks:** Test overload ambiguity, direction changes, null arrays, differing widths, extreme counts, and per-signature reference results.
- **Done When:** Each helper is native and behaves correctly when selected through normal package visibility.
- **Do Not:** Invent logical/reduction overloads absent from the source inventory.

### [ ] 097 - Gate Bit-Unsigned Package Coverage

- **Work:** Reconcile every `numeric_bit_unsigned` signature, alias, dependency, and native binding with the inventory.
- **Checks:** Run the full per-signature corpus, designs importing competing numeric packages, both artifact formats, and body-free deployment.
- **Done When:** Every required wrapper row is implemented and no interpreted standard-body fallback is reachable.
- **Do Not:** Infer full coverage merely because the underlying `numeric_bit` package passes its own tests.

### [ ] 098 - Add Logic-Unsigned Interfaces and Conversions

- **Work:** Install `numeric_std_unsigned` declarations and implement its logic-vector/integer conversion and resize adapters over `numeric_std`.
- **Checks:** Cover exact resolved/unresolved types, aliases/defaults, nonbinary values, null arrays, directions, and wide conversions.
- **Done When:** Wrapper conversion profiles have native identities and preserve required logic-state behavior.
- **Do Not:** Collapse all logic vectors to two-state integers to share the binary fast path.

### [ ] 099 - Add Logic-Unsigned Arithmetic Adapters

- **Work:** Implement declared arithmetic wrappers through specialized `numeric_std` kernels with explicit operand/result mapping.
- **Checks:** Exercise all scalar/vector combinations, sizing, wide arithmetic, zero divisors, warnings, and nonbinary propagation.
- **Done When:** Each arithmetic signature matches its 2008 definition without package-body execution.
- **Do Not:** Lose the wrapper's result bounds or diagnostics while delegating.

### [ ] 100 - Add Logic-Unsigned Comparison Adapters

- **Work:** Implement declared comparisons/matching, extrema, and search helpers through existing typed numeric dispatch.
- **Checks:** Cover unequal lengths, null vectors, resolved/unresolved associations, metavalues, defaults, and competing visible overloads.
- **Done When:** Every comparison/helper family assigned here has per-signature native coverage.
- **Do Not:** Order logic-state enum discriminants to implement numeric comparisons.

### [ ] 101 - Add Logic-Unsigned Shift Adapters

- **Work:** Implement the declared shift/rotate helpers and remaining non-arithmetic transformations, reusing native `numeric_std` kernels with wrapper-specific constraints.
- **Checks:** Test negative counts where legal, oversized counts, null vectors, direction/width preservation, and metavalues.
- **Done When:** All remaining declared transformation signatures match the pinned 2008 profiles.
- **Do Not:** Add similarly named shift operators that are not declared by this package.

### [ ] 102 - Gate Logic-Unsigned Package Coverage

- **Work:** Audit every `numeric_std_unsigned` declaration, overload, alias, dependency, and binding, creating bounded children for remaining gaps.
- **Checks:** Run per-signature and mixed-package tests, binary/RON fresh-load workflows, and source/body-free deployment.
- **Done When:** The complete declared wrapper interface has tested native behavior and no hidden fallback.
- **Do Not:** Count underlying `numeric_std` coverage as proof of wrapper overload/visibility correctness.

### [ ] 103 - Add IEEE Compatibility and Context Units

- **Work:** Register the 2008 `std_logic_textio` compatibility package and `ieee_std_context`/`ieee_bit_context` declarations with exact dependencies and visibility. Reuse context analysis and library artifacts rather than introducing executable stubs.
- **Checks:** Analyze designs using each context/compatibility unit, check imported names and conflicts, reload artifacts, and verify no legacy body or duplicate I/O declarations are loaded.
- **Done When:** Empty compatibility units and context units are independently inventoried, analyzable, and reproducible in both artifact formats.
- **Do Not:** Copy older `std_logic_textio` declarations or silently import Synopsys/VITAL packages.

### [ ] 104 - Gate All IEEE Non-I/O Cores

- **Work:** Reconcile all thirteen IEEE package surfaces, both contexts, generic policies, dependency identities, and completed non-I/O native signatures. Retain every text-I/O signature for `108` and final closure in `122`.
- **Checks:** Run the expanded source audit, package-specific/reference suites, cross-package generic designs, compatibility/context tests, and both encodings without standard bodies.
- **Done When:** Every non-I/O IEEE row is implemented and tested, with no silent omissions or interpreted fallback. Pending I/O is explicitly not claimed complete.
- **Do Not:** Treat directory names, equal overload counts, or package-level smoke tests as evidence of full conformance.

## H. Remaining Language Coverage and Hardening

### [ ] 105 - Implement Access Types and Allocation

- **Work:** Add access-type identity, nulls, allocation, designated subtype checks, dereference, deallocation, and lifetime/storage semantics from the matrix. Use safe runtime handles and diagnostics for invalid operations.
- **Checks:** Cover nested/composite designated values, aliasing, null dereference, deallocation, scope/lifetime cases, and repeated instances. Persist declarations/initialization plans, not live host pointers.
- **Done When:** Required access operations execute with explicit ownership/handle rules and source-linked failures.
- **Do Not:** Serialize runtime addresses or equate Rust ownership rules directly with VHDL access semantics.

### [ ] 106 - Implement Language File Objects and Operations

- **Work:** Add file-type declarations, object/open/close lifetime, modes/status, read/write/endfile, and relevant standard operations according to the matrix. Document external file representation and I/O-error policies where implementation-defined.
- **Checks:** Use temporary test directories for normal/empty/missing files, wrong modes, EOF, composite element constraints, and cleanup. Verify binary/RON artifacts contain plans, not open handles or build-machine absolute paths.
- **Done When:** VHDL file semantics are distinct from raw Rust I/O and errors retain VHDL source context.
- **Do Not:** Perform simulation file effects during analysis or artifact inspection.

### [ ] 107 - Implement Core Text I/O

- **Work:** Implement the required VHDL 2008 `std.textio` declarations and native line/scalar read/write/readline/writeline families, including standard input/output objects and applicable formatting/status helpers. Use access/file facilities rather than a disconnected output mechanism; remaining binary/octal/hex and alias families are completed in chunk `108`.
- **Checks:** Cover whitespace, empty lines, conversion failures, good/status overloads, field formatting, line lifetime, and comparison with reference fixtures without a text I/O package body.
- **Done When:** Core text I/O has inventoried native signatures and deterministic fixture output.
- **Do Not:** Treat `report` as a replacement for language file/text I/O semantics.

### [ ] 108 - Complete All Standard Package I/O Families

- **Work:** Complete every inventoried VHDL 2008 `std.textio` and IEEE package I/O signature, including logic/numeric/bit/fixed/floating read/write and binary/octal/hex/alias families where declared. Split these package families into ordered children rather than implementing them all in one change. Reuse access/file/line primitives and retain each package's exact profile, states, formatting, and warning policy.
- **Checks:** Cover nonbinary digits, widths/directions, partial input, formatting, invalid characters, and all required overloads. Check body-free execution and artifact parity.
- **Done When:** Every required standard-package I/O row has a tested native implementation, with no package-body dependency or silently missing overload.
- **Do Not:** Hide unfinished package I/O behind a completed core-package gate or load a legacy compatibility body.

### [ ] 109 - Implement Protected Types and Shared Variables

- **Work:** Add protected declarations/bodies, protected method calls, shared-variable restrictions, per-instance storage, and atomic execution semantics. Enforce restrictions on waits, visibility, and method bodies.
- **Checks:** Use competing processes, nested permitted calls, separate instances, invalid shared declarations, and prohibited suspended operations. Check deterministic behavior only where the language guarantees it.
- **Done When:** Shared state follows VHDL protected semantics rather than arbitrary host thread locking.
- **Do Not:** Introduce host parallelism or permit unconstrained shared mutation to avoid implementing the language rules.

### [ ] 110 - Add Aliases, User Attributes, and External Names

- **Work:** Implement remaining alias forms, user-defined attribute declarations/specifications/lookup, and VHDL 2008 external names with hierarchy-aware elaboration and subtype checks. Preserve references across linked/bundled artifacts.
- **Checks:** Cover object/subprogram aliases, attribute placement/type errors, generated-instance paths, valid/invalid external references, and diagnostics relating use sites to targets.
- **Done When:** These references bind to semantic identities and correct elaborated objects rather than fragile name strings.
- **Do Not:** Resolve hierarchy-dependent names prematurely or let aliases bypass type/driver restrictions.

### [ ] 111 - Complete VHDL 2008 Expression/Statement Extensions

- **Work:** Reconcile matching operators/case forms, condition conversion, and conditional/selected assignment/expression forms against the pinned 2008 grammar and feature matrix. Add each missing family in bounded children, sharing existing typing/control-flow machinery.
- **Checks:** Include positive/negative revision tests, matching metavalues, type/context ambiguities, sequential/concurrent forms where permitted, and optimized/unoptimized equivalence.
- **Done When:** Every extension row assigned to this chunk is implemented according to VHDL 2008, not a later revision's grammar.
- **Do Not:** Assume parser acceptance proves a construct belongs to 2008 or give matching equality ordinary equality semantics.

### [ ] 112 - Implement Force and Release Semantics

- **Work:** Add VHDL 2008 force/release assignments and effective/driving value behavior for applicable signal modes. Integrate with drivers, resolution, ports, events, and attributes.
- **Checks:** Test forced resolved/unresolved signals, propagation through hierarchy, release timing, continuing driver transactions under force, and illegal target/mode cases.
- **Done When:** Force/release behavior is explicit in the runtime and matches observable event/history semantics.
- **Do Not:** Implement force as destructive replacement of the underlying driver's pending waveform.

### [ ] 113 - Define Waveform Registry and Observation Contract

- **Work:** Define a typed waveform sink/registry boundary over effective signal updates, independent of artifact codecs and runtime values. Specify hierarchical names, aliases, ranges, signal selection, initial values, integer timescales, delta ordering, and type/state mappings for VCD and FST.
- **Checks:** Use small VHDL fixtures and captured events for initialization, same-time changes, resolved drivers, aliases, hierarchy, and ascending/descending vectors. Verify tracing disabled does not retain signal histories or alter semantic traces.
- **Done When:** Both backends can consume one tested event/metadata contract, with explicit policies for objects/types that cannot be dumped.
- **Do Not:** Observe transient per-driver intermediate values as effective changes, round event times silently, or modify simulation values to fit a waveform format.

### [ ] 114 - Implement VCD Waveform Writing

- **Work:** Integrate an established Rust VCD writer where suitable with scopes, stable identifiers, declaration ranges, timescale, initial dump, and streamed changes. Implement explicit VHDL-to-VCD scalar/vector/real mappings and checked output errors.
- **Checks:** Read generated VCD with an independent parser; compare declarations, timestamps, initial values, changes, escaped names, null/large vectors, weak/unknown states, and stop-time boundaries. Normalize only irrelevant headers/paths in golden tests.
- **Done When:** A saved elaborated design produces a standards-readable VCD file without retaining the entire simulation history.
- **Do Not:** Hand-roll a waveform grammar unnecessarily, silently collapse unsupported states, or introduce artificial positive delays to represent delta cycles.

### [ ] 115 - Implement FST Using fst_writer

- **Work:** Verify and pin the `fst-writer` Cargo dependency, imported as `fst_writer`, and implement FST scopes/variables, timescale, initial values, streamed changes, compression/finalization, and error mapping through the shared contract. Check supported APIs, license, toolchain, and type/width limits before binding them.
- **Checks:** Read FST with an independent compatible reader or pinned `fst2vcd` utility, covering hierarchy, aliases, wide/nonbinary values, same-time changes, and finalization. Compare decoded observations with the shared event sink and document any actual format/reader limits.
- **Done When:** FST output is produced by `fst_writer`, opens in an independent reader, and has verified times/values rather than merely a nonempty file.
- **Do Not:** Write VCD text under an FST extension, substitute another backend silently, or claim byte-identical compressed output across platforms.

### [ ] 116 - Add Waveform CLI and Output Lifecycle

- **Work:** Add `simulate --vcd PATH` and `--fst PATH`, allowing either or both, plus hierarchical signal/depth filters. Validate collisions with input/other output paths and define safe file-creation behavior. Finalize enabled writers on natural completion, stop/finish, limits, and assertion failure; report I/O errors with meaningful exit status.
- **Checks:** Exercise fresh-process binary/RON input combinations, each output alone and simultaneous output, filtered hierarchies, invalid paths, initialization failures, and interrupted/error exits where cleanup is possible.
- **Done When:** Waveform output is optional, predictable, and usable from the ordinary simulation CLI without reparsing VHDL or changing execution semantics.
- **Do Not:** Overwrite an input artifact, mask simulation failures with a successful close, or promise complete output after an uncatchable process kill.

### [ ] 117 - Complete Waveform Value and Boundary Coverage

- **Work:** Complete dump mappings for the supported scalar, enumeration, physical, array/record, logic/numeric, fixed/floating, and complex signal values. Flatten/encode composites with documented metadata and handle unsupported access/file/protected objects explicitly. Split independent type families into children if needed.
- **Checks:** Cover packed/big/nonbinary storage, all nine logic states and documented output mappings, null/ascending/descending arrays, enum labels, real values, hierarchy aliases, force/release, and zero-time activity. Compare dump-enabled and disabled simulation results.
- **Done When:** Every waveform type/shape policy has positive and negative/boundary fixtures, and format limitations are visible rather than silently approximated.
- **Do Not:** Treat binary storage optimizations as a reason to truncate widths or lose source index/type information.

### [ ] 118 - Gate Waveform Parity in GitHub Actions

- **Work:** Add required GitHub Actions waveform jobs with pinned readers/reference utilities. Decode VCD and FST into normalized signal observations and compare them with expected fixtures, each other where their formats permit, and equivalent reference-simulator traces. Preserve useful failure artifacts.
- **Checks:** Run binary/RON and linked/bundled workflows, multiple timescales, all dump modes, platform/path checks, clean termination, deliberate corrupt-output failures, and a missing-reader failure. Check same-time event ordering where supported and document reader/format limitations without inventing timestamps.
- **Done When:** Both waveform formats have independently validated values/times and required CI coverage; absent tools cannot produce a passing waveform gate.
- **Do Not:** Rely only on file sizes, raw compressed-byte snapshots, or a successful writer return code.

### [ ] 119 - Reconcile Remaining Language-Matrix Rows

- **Work:** Audit all matrix rows, including concurrent statements and their process lowering, declarations, attributes, staticness, configurations, and less common object/type rules. Give each gap a bounded child task or an explicit approved scope decision before advancing.
- **Checks:** Require a positive and negative/edge fixture for each required row, with the correct stage outcome. Cross-check that constructs accepted by analysis either elaborate/execute correctly or receive an explicit documented diagnostic.
- **Done When:** No required language feature is only parsed, silently ignored, or supported accidentally by a different path.
- **Do Not:** Label the simulator fully VHDL 2008 conformant while required rows are deferred; foreign interfaces and optional external integrations need separate scope decisions.

### [ ] 120 - Audit Diagnostic Quality Across All Stages

- **Work:** Upgrade representative syntax/name/type/width/overload/generic/binding/runtime/artifact failures to the shared multi-level diagnostic standard. Add related declaration/operand labels, concrete bounds, instance/call context, and actionable notes without overwhelming output.
- **Checks:** Snapshot multi-file and source-unavailable cases, ambiguous overloads, generic-induced widths, runtime bounds/assertions, and native package domain errors. Verify stable codes and structured data separately from terminal styling.
- **Done When:** Diagnostics explain what failed, where, and why at every stage, including loaded artifacts.
- **Do Not:** replace useful underlying context with a generic serialization/runtime error or leak a panic as the user diagnostic.

### [ ] 121 - Harden Artifact Decoding and Compatibility

- **Work:** Extend binary/RON validation to the now-complete schema: sizes/depths, operation IDs, source/type references, malformed big values, dependency/intrinsic versions, and impossible executable states. Define old-version rejection and future migration policy.
- **Checks:** Fuzz/truncate/mutate both codecs; test resource limits, wrong-stage files, stale catalogs, incompatible policy/runtime versions, and atomic write recovery. Retain regression fixtures for every found crash.
- **Done When:** Malformed artifacts fail before unsafe execution, with bounded resource use and structured errors.
- **Do Not:** Trust RON because it is readable, deserialize host pointers, or promise compatibility that has not been tested.

### [ ] 122 - Gate Complete Native Standard-Library Independence

- **Work:** Regenerate/audit the standard declaration-to-intrinsic coverage report for all three `std` packages, all thirteen IEEE packages, and both IEEE contexts, including every relevant default, overload, alias, policy, I/O signature, and transitive dependency. Build a restricted test environment with their VHDL package bodies absent and interpreted fallback disabled.
- **Checks:** Run direct native calls, elaboration-time evaluation, generic package instances, wide/nonbinary values, and mixed-package end-to-end designs in both formats.
- **Done When:** Every required native signature is accounted for and the simulator never requests the corresponding VHDL package body.
- **Do Not:** Count a name as covered while one overload, policy instance, or required dependency still uses a body fallback.

### [ ] 123 - Gate Functional Conformance Before Optimization

- **Work:** Freeze the current required-scope matrix and collect analysis, elaboration, scheduling, package, I/O, hierarchy, waveform, and diagnostics suites into required GitHub Actions gates. Publish remaining documented limitations without presenting them as supported behavior.
- **Checks:** Run direct/property/differential suites, fresh-process workflows, linked/bundled artifacts, source-free simulation, both formats, and full milestone checks. Resolve required-scope failures before continuing.
- **Done When:** The committed functional surface is green and any non-goals/limitations are explicit and reviewed.
- **Do Not:** Trade a known semantic failure for performance or use broad ignored-test groups to pass the gate.

## I. Performance, Robustness, and Release

### [ ] 124 - Establish Reproducible Performance Baselines

- **Work:** Add benchmarks and profiling instructions for process dispatch, arithmetic/vector widths, nonbinary transitions, event density, driver resolution, artifact I/O, hierarchy elaboration, and VCD/FST dumping on/off. Record time, allocation, memory, and operation counts with toolchain/platform/workload metadata.
- **Checks:** Ensure every benchmark's result is checked against the functional model. Repeat runs to identify noise and set justified regression thresholds rather than universal speed claims.
- **Done When:** The next optimizations have measurable hypotheses and a stable baseline, including binary versus RON costs.
- **Do Not:** use a no-op simulation as proof of performance or require RON to equal binary throughput.

### [ ] 125 - Optimize Measured Dispatch Hot Paths

- **Work:** Optimize operation selection, specialized kernel binding, process activation, and call-frame layout where the profile shows benefit. Cache by concrete type/width/policy and keep overload/name resolution out of execution. If native code was required in chunk 003, optimize that existing backend rather than changing the requirement here.
- **Checks:** Compare operation counts, allocation, and dispatch benchmarks before/after; run affected conformance suites and artifact reload tests. Confirm no executable addresses enter the persisted format.
- **Done When:** A measured improvement survives correctness tests without proliferating unjustified special cases.
- **Do Not:** Introduce unsafe code, generated variants, or a new backend without evidence and a bounded verification plan.

### [ ] 126 - Optimize Measured Vector and Numeric Paths

- **Work:** Reduce allocation/copying and improve small/big/fallback transitions, wide arithmetic, and common fixed/float operations according to profiles. Reuse suitable numerical primitives; retain a tested general path for all required cases.
- **Checks:** Run representation-equivalence properties, widths around every specialization threshold, unknown-state cases, signed/policy boundaries, and before/after memory/throughput benchmarks.
- **Done When:** Binary-only common values take the intended efficient path and optimizations remain semantically transparent.
- **Do Not:** Optimize by changing widths, dropping leading zeros, erasing metavalues, or weakening rounding guarantees.

### [ ] 127 - Optimize the Scheduler Against Real Event Profiles

- **Work:** Optimize queue organization, cancellation, sensitivity registration, driver batching, and temporary allocations only where measured. Keep time/delta/phase ordering and VHDL transaction rules unchanged.
- **Checks:** Compare sparse/dense clocks, combinational delta chains, inertial cancellation, high-fanout signals, and multi-driver workloads; rerun the complete scheduling differential gate.
- **Done When:** Measured scheduler improvements retain identical normalized semantic traces and bounded memory behavior.
- **Do Not:** Substitute wall-clock concurrency for event semantics or drop cancelled-event bookkeeping without proving correctness.

### [ ] 128 - Run Fuzzing, Stress, and Deterministic Replay

- **Work:** Add bounded syntax-to-analysis inputs, typed-IR/value properties, artifact mutations, generated small designs, and long-running event stress cases. Record seeds and minimal reproducers; use time/memory limits without classifying a limit as a successful simulation.
- **Checks:** Replay failures deterministically; compare small generated legal designs with the reference; test no-panics/error containment at parser, semantic, artifact, and runtime boundaries. Check all encodings and key storage variants.
- **Done When:** A documented fuzz/stress budget passes and every discovered relevant defect has a focused regression.
- **Do Not:** Expose unbounded arbitrary native execution through fuzzed artifact operation IDs or discard failing seeds.

### [ ] 129 - Document the Three-Stage User Workflow

- **Work:** Write working examples for libraries, top/architecture/configuration selection, generics, linked/bundled artifacts, simulation limits, diagnostics, native package support, binary/RON inspection/conversion, VCD/FST output/filtering, and GitHub Actions checks. Document supported revision, implementation limits, waveform mappings, reproducibility, and compatibility policy.
- **Checks:** Execute documented commands from a fresh directory, verify default binary output and explicit RON output, and reproduce source-free simulation and error examples.
- **Done When:** A user can build, inspect, and simulate a small design without relying on agent chat history.
- **Do Not:** Describe planned/deferred functionality as released or imply that editing arbitrary RON bypasses validation.

### [ ] 130 - Run the Release Acceptance Gate

- **Work:** Freeze/version the initial supported schema and native declaration inventory, produce release artifacts/instructions for supported targets, and publish the verified support matrix and measured limitations. Verify licensing/attribution for dependencies and standard declarations without introducing standard-body runtime dependencies.
- **Checks:** Run clean-build formatting/compilation/Clippy/tests, required differential suites, all package coverage reports, artifact corruption/round-trip/compatibility checks, independently decoded VCD/FST tests, benchmark regressions, documented CLI examples, and source/body-free deployment tests in GitHub Actions. Verify reproducible semantic artifact identities on supported environments.
- **Done When:** Every committed project requirement is demonstrated by a named passing gate, or a remaining scope decision is explicit and approved before release. Binary is the default and RON remains a first-class inspectable, loadable alternative.
- **Do Not:** Claim complete standard conformance, cross-version compatibility, cross-platform bit identity, or a native-code backend beyond what the evidence supports.

## Project-Level Acceptance Checklist

- [ ] `vhdl_syntax` is the verified parser dependency and only VHDL 2008 is selectable.
- [ ] Analysis, elaboration, and simulation work as independent processes with explicit artifacts between them.
- [ ] Each primary unit has its own analyzed artifact; secondary units and exact dependency versions are preserved.
- [ ] Type/width errors are caught at the earliest valid stage, including generic-dependent elaboration checks and required runtime checks.
- [ ] Elaborated designs contain specialized executable behavior, not parser ASTs or runtime overload searches.
- [ ] Event scheduling, waits, drivers, resolution, waveforms, delta cycles, postponed behavior, and attributes pass the required conformance suite.
- [ ] Every VHDL 2008 `std.standard`, `std.textio`, and `std.env` signature and all thirteen IEEE package signatures execute natively without their VHDL package bodies, including policies, aliases, and I/O; both IEEE context units and the compatibility package have verified analysis/visibility behavior.
- [ ] Binary-only vectors use small/big integer paths; nonbinary values are preserved losslessly; index directions and signedness stay correct.
- [ ] Both stage artifacts have binary and readable RON encodings of one versioned logical schema.
- [ ] Both-format round trips, conversion, deterministic identity, corruption rejection, compatibility checks, and identical simulation results are tested.
- [ ] `ariadne` diagnostics include related locations/types/bounds and useful elaboration/runtime context, including from saved artifacts.
- [ ] Simulation optionally generates independently readable VCD and FST files, using `fst_writer` for FST, with verified hierarchical selection, initial values, timescales, changes, finalization, and documented type/delta limitations.
- [ ] GitHub Actions pull-request/push workflows enforce the required Rust, reference, native-library, waveform, and cross-platform diagnostic checks without silently skipping missing tools.
- [ ] A fresh deployment can run linked or bundled elaborated designs without original source files or standard package bodies.
- [ ] The feature matrix, per-signature native inventory, implementation limits, benchmark evidence, and release claims agree.
