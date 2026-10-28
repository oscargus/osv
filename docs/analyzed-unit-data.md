# Analyzed Object and Process Data

Plan chunk `019`: owned-data child `019a` and source-analysis child `019b`.
[../src/analysis/unit_data.rs](../src/analysis/unit_data.rs) defines owned
declaration data embedded in the analysis payload. Validation is isolated in
[../src/artifacts/unit_data.rs](../src/artifacts/unit_data.rs).

`ObjectDeclaration` links a resolved symbol to `ObjectInitialization::Default`
with a typed primitive value or `Expression` with an owned flat plan. The
analysis payload preserves object order, declaration IDs, exact source/type
references, and explicit initialization choice. It does not allocate drivers,
execute an initializer, or treat a signal like an immediately assigned variable.
Constant objects require an expression rather than an invented default.

`PendingProcess` retains optional label, lexical `ScopeId`, and full source
`ByteSpan`. It is an explicit, nonexecutable boundary awaiting chunk `020`,
not an empty process, no-op, interpreter closure, or discarded body. Source
records already retain the corresponding bytes for later analysis/diagnostics.
No parser AST is persisted.

Artifact schema `4` adds `AnalysisArtifact::objects` and `pending_processes`.
Older schemas are rejected deliberately. Compiler/native epochs and native
declaration manifest semantics are unchanged. Object/process vectors keep
semantic source order; canonicalization does not reorder them as lookup tables.

Validation checks object identity/ownership/kind/declaration membership,
duplicate objects, initialization type/range, expression result type, and all
embedded-plan references. Pending processes require existing source spans and
distinct same-unit lexical scopes. Expression validation now separates full
envelope validation from `validate_in_context`, preventing recursion when an
envelope contains expression plans. This is structural typed-data validation,
not hidden source reanalysis or process execution.

Codecs can inspect/convert structurally valid pending analysis records without
changing them. `LibraryStore::publish` refuses such records with explicit
`IncompleteAnalysis`, before creating store files or updating a catalog.
Exact/current graph loading also rejects a pending artifact if a catalog is
externally forged to reference it. No pending process can count as a successful
analyzed library entry; `020` must supply actual typed process data before the
normal publication path can proceed.

The synthetic tests in [../tests/unit_data.rs](../tests/unit_data.rs) verify
default/expression object and pending-process round trips, complete identity
preservation, no recursive validation, bad symbol/source/scope references,
duplicate/default-kind/type errors, malformed embedded plans, and the no-files
incomplete-publication barrier. These are model fixtures; real source coverage
is separate from process analysis, which remains `020`.

## Source Unit Analysis

`analysis::analyze_units(&SourceMap, library)` in
[../src/analysis/source_units.rs](../src/analysis/source_units.rs) consumes raw
shared source records in stable source-ID order and returns an owned artifact
for each supported entity or architecture. Primary declarations must precede
their secondaries. There is no filesystem source lookup or package-body load.
Syntax views remain transient; no parser node survives in the returned data.

The initial source subset is portless entities and associated architectures
with scalar constants/signals. Boolean, integer, time, natural, positive, and
delay-length subtype names resolve through installed native declarations.
Initializers use the typed expression analyzer with expected types and known
earlier constants, including inherited entity constants. Objects without an
explicit signal initializer retain a typed default: false for boolean, the
declared signed minimum for integer/time, zero for natural/delay-length, and
one for positive. Constants cannot receive an invented default.

Architecture artifacts retain distinct secondary identity, primary scope and
declarations, and the exact primary artifact fingerprint. The dependency is
attached before any validated prelude/expression operation. Unneeded unrelated
unit/scope/source records are removed after the necessary context is retained.
Object/declaration order and original Latin-1 bytes/spans remain intact.

Labelled or unlabelled architecture processes become explicit `PendingProcess`
records with their actual lexical scope and source extent. Bodies are neither
executed nor dropped; the publication barrier remains in force. `020` must
analyze their statements and produce real typed process data. Passive entity
statements, other concurrent statements, hierarchy, ports, generics, generate,
guarded bus/register signals, user types, and later declaration families are
source-linked unsupported errors rather than no-ops. Full package/context
visibility and wider scalar/composite coverage remain their later owners.

Duplicate objects, absent/wrong primary units, mismatched ending names,
unknown types, missing constant initializers, and initializer type/range errors
produce shared analysis diagnostics. Failed calls return no partial artifact
batch. This API does not publish artifacts or implement the `analyze` command;
the command/catalog integration remains `021`, after process analysis.

[../tests/source_units.rs](../tests/source_units.rs) uses real VHDL fixtures for
empty entities and an entity/architecture design with inherited constants,
initial/default signals, and a labelled process. It checks dependency identity,
both-codec owned results, pending-publication rejection, and the negative
source/association/declaration/type/interface cases. These tests do not claim
elaboration or simulation functionality.

```sh
cargo test --test source_units --test unit_data --locked
cargo fmt --all
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
```
