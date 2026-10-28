# Analyzed Object and Process Data

Plan child `019a`; source-driven unit analysis remains `019b`.
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
incomplete-publication barrier. These tests are model fixtures, not a claim
that `019b` or process analysis is implemented.

```sh
cargo test --test unit_data --locked
cargo fmt --all
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
```
