# Owned Artifact Model

Plan chunk `007`. The production model is in
[../src/artifacts/model.rs](../src/artifacts/model.rs), with validation in
[../src/artifacts/validate.rs](../src/artifacts/validate.rs) and canonical
identity in [../src/artifacts/identity.rs](../src/artifacts/identity.rs).
File framing, bounded input readers, and codec diagnostics belong to `008`;
commands and library catalogs belong to `009` and `010`.

## Envelope and References

`ArtifactEnvelope` carries schema version `4`, compiler compatibility epoch
`1`, a diagnostic compiler-version string, the sole `Vhdl2008` revision,
logical unit identity, source/unit/type/symbol tables, exact dependencies,
native declaration-set fingerprints and ABI requirements, and a typed payload.
Its stage is derived from `ArtifactPayload::Analysis` or `Elaboration`, never
an independently mutable string or tag. `validate(expected_stage)` checks
compatibility and structure without invoking any compiler stage.

`UnitId`, `TypeId`, `SymbolId`, `ScopeId`, and the existing `SourceId` are distinct fixed
`u32` identities local to their artifact tables. They are references, not array
indices, paths, or host addresses. Owners must allocate them deterministically
and retain them across persistence; canonicalization never silently renumbers
them. Sparse IDs are legal. Table insertion order is not identity. Changing an
ID and its references is an explicit payload change, not automatic alpha
renaming.

`UnitIdentity` records library, primary name, and a typed unit kind. An
architecture carries its own name; a package body is a distinct secondary
kind. Secondary records reference the appropriate entity/package parent with
matching library/primary identity. A secondary analysis artifact must record
its exact primary declaration dependency. Dependencies distinguish declaration,
body, and elaborated-design roles and retain stage, fingerprint, schema, and
compiler requirements. The later store verifies actual graph availability.

Identifiers distinguish basic and extended names. Basic names normalize to
lowercase; extended names retain case. Names must be nonempty Latin-1 text;
full lexical, reserved-word, visibility, and declaration rules remain owned by
the frontend. Validation happens before normalization, so a non-Latin-1 input
cannot become accepted through Unicode case folding. Unit identities and
native/dependency keys are checked after name normalization as well.

Schema `3` also distinguishes character literal and typed operator names and
adds `TypeDefinition::Native` plus explicit per-symbol native bindings. See
[standard-prelude.md](standard-prelude.md). Native identities require the exact
trusted standard unit/declaration-set/ABI and validated type/signature metadata;
they are not inferred from spelling. Previous schemas are rejected explicitly.

The initial type records describe scalar, enumeration, and unconstrained
subtype identity, not primitive arithmetic or runtime storage. Symbols retain
unit/type identity and source sites; optional type-declaration links are
checked in both directions. Enumeration order is meaningful. Subtype cycles
and dangling references fail; checked subtype walks reuse completed paths.
The analysis payload selects one unit and preserves its ordered declaration
IDs. The elaboration payload selects a root entity and its actual architecture.
Future chunks extend these records deliberately with typed expressions,
layouts, continuations, and operations. These initial synthetic artifacts are
not executable designs and cannot be simulated by the bootstrap CLI.

## Lexical Scopes

Chunk `016a` adds owned `ScopeRecord` data with typed `ScopeId` references and
`ScopeKind::{Library, Unit, Lexical}`. Each unit has exactly one unit scope;
primary units parent to the matching library scope, and secondary units to
their primary's scope. Nested lexical scopes retain an owning unit and parent
within that same unit. Symbols retain both declaration unit and lexical scope.
Validation rejects duplicate/dangling scope IDs, missing/duplicate unit scopes,
wrong library or symbol ownership, invalid parents, and cycles. Traversal is
iterative with completed-path reuse; sparse IDs remain unchanged on reload.

Scope records are canonicalized by typed ID, library names are normalized,
and scope data participates in logical identity. These fields are incompatible
with the original schema `1`, which is deliberately rejected rather than
silently migrated or filled with guessed default scopes. Compiler epoch and
native ABI remain `1`. This is persistence/validation only: name lookup and
syntax registration are implemented separately by child `016b`; see
[scopes-and-names.md](scopes-and-names.md). These do not imply type checking or
an implemented analysis command.

## Analyzed Unit Data

Schema `4` adds typed object initialization and explicit pending-process
source/scope records to analysis payloads. Embedded expression validation uses
a prevalidated context boundary rather than recursively validating the same
envelope. Pending data is inspectable/convertible but cannot be published as
a successful library entry. See [analyzed-unit-data.md](analyzed-unit-data.md).
Source unit analysis is implemented by `019b`; process statement analysis
remains `020`, with pending boundaries explicitly unpublishable.

## Source Fidelity

`SourceRecord` wraps the unchanged shared `SourceFile` plus a BLAKE3 content
fingerprint. `SourceRecord::embedded` fingerprints original Latin-1 bytes;
validation checks embedded bytes against the saved fingerprint and the source
offset limit. Missing bytes are permitted when restoring an already
fingerprinted record, with no attempt to reopen its display name. Available
source spans must be in bounds; unavailable sources still require an existing
source ID and nonreversed offsets. Source bytes, IDs, names, and fingerprints
survive persistence. Display-only path separators normalize to `/`.

## Canonical Identity

`canonicalized()` returns an independently owned, validated copy. It normalizes
basic names and display separators, sorts source/unit/type/symbol tables by
typed ID, dependencies by identity/role/stage, and native requirements by
library/package. It rejects duplicate IDs, duplicate normalized unit identities,
duplicate dependencies/native requirements, invalid parent/type/symbol links,
invalid spans, incompatible versions/ABIs, and incorrect payload identity.
It does not sort declaration sequences or enumeration literals.

`fingerprint()` computes BLAKE3-256 over Postcard encoding of that normalized
envelope, clearing only the diagnostic compiler-version string. Schema and
compiler compatibility, source data, names/types/symbols, dependencies/native
requirements, stage and payload remain included. External binary framing,
RON whitespace, and compiler display-version spelling are excluded. The
fingerprint uses fixed 32-byte data, not a string ID; its display form is 64
lowercase hexadecimal digits. Original records are never mutated.

The model contains only owned data, fixed-width IDs/scalars, typed enums,
optional records, strings, and ordered vectors. It contains no parser node,
arena, pointer, closure, live process/driver state, file handle, or dispatch
function. Serde derives support the same model in Postcard and RON; production
loaders must additionally impose the input/allocation/depth limits from
[execution-and-artifacts.md](execution-and-artifacts.md).

Schema field and enum ordering are persisted contract, including Postcard
variant numbers. Incompatible changes require a schema bump and explicit
rejection, not best-effort loading. No migration or cross-release promise is
made. Native ABI `1` is an initial requirement tag, not a claim that the native
registry or any standard package implementation exists. No VHDL package body
is read or required.

## Verification

[../tests/artifacts.rs](../tests/artifacts.rs) seeds explicitly synthetic
analysis/elaboration records and checks stage/version rejection, all four
duplicate/reference tables, real insertion-order permutations, significant
declaration/enumeration order, basic/extended case rules, source fidelity and
missing-source behavior, secondary primary dependencies, subtype cycles,
native ABI failures, both-codec equality, and readable named RON fields.

```sh
cargo test --test artifacts --locked
cargo fmt --all
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
```
