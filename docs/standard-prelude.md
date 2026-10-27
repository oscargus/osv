# Trusted Standard Prelude

Plan child `017b`, completing parent `017` after the identity work in
[intrinsic-identity.md](intrinsic-identity.md). Installation lives in
[../src/intrinsics/prelude.rs](../src/intrinsics/prelude.rs); owned binding
validation lives in [../src/artifacts/native.rs](../src/artifacts/native.rs).
No VHDL standard-package source or body is loaded.

## Owned Declarations

`standard_prelude()` returns a validated owned analysis artifact for the
native `std.standard` package. It contains library/package scopes, eighteen
type/subtype declarations, exact enumeration literals (including all 256
character positions), time units, the foreign attribute declaration, and the
selected initial operator/function profiles from `017a`. Source tables are
empty, declaration locations are absent rather than invented, and there is no
package-body unit. Report/assert remain statement identities in the registry,
not invented exported functions. `now` retains its time-observation metadata.

Artifact schema `3` adds typed native type references, explicit native symbol
bindings, and distinct character/operator names. `Identifier::Character` does
not case-fold like a basic identifier, and `Identifier::Operator` cannot be
confused with a user extended identifier of the same visible spelling.
`TypeDefinition::Native(NativeType)` refers to the versioned registry profile;
its precise scalar/subtype/array shape is not duplicated as lossy generic
strings. Type/enum-literal/physical-unit/attribute/subprogram symbol kinds and
`NativeBinding` preserve actual declaration roles and intrinsic identities.
Older schemas `1` and `2` are rejected, with no silent migration or guessed
native fields. Compiler compatibility and native ABI remain `1`.

## Installation and Lookup

`install_standard(&mut artifact)` validates input, builds the trusted prelude,
and transactionally adds its immutable records. Existing source/unit/type/scope/
symbol IDs remain unchanged; new IDs allocate above the actual high-water mark
with checked arithmetic. Existing `std` library scopes are reused. The returned
`PreludeIds` maps native type identities to local type IDs and identifies the
package scope. Repeated installation is idempotent only for a complete trusted
prelude; incomplete/conflicting declarations fail explicitly. Exhaustion leaves
the original artifact untouched.

`ScopeLookup::lookup_with_standard` first performs ordinary lexical lookup,
then searches the installed standard package only if no lexical candidate was
found. User homonyms therefore shadow normally and are not rewritten as native
calls. Later expression analysis must use resolved symbol/type/native records,
not infer semantics from names. No package visibility/import implementation is
smuggled into this fallback; full `use` and context rules keep their later owners.

Native records require the exact trusted unit, declaration-set fingerprint,
ABI, package scope, type/literal/unit/profile metadata, and intrinsic reference.
Validation rejects duplicate bindings, mismatched type names, forged user-unit
bindings, wrong IDs/profiles/results, incorrect units/literal positions, and
stale requirements. Persisted native basic names normalize during canonical
identity generation. There is no serialized function pointer or live host state.

## Scope Limits

Installed declarations do not imply every standard operation is executable.
Scalar real/physical arithmetic, composite operations, edge helpers, file/I/O,
and complete predefined coverage remain assigned to `031`/`040`/`057`/`105`-`108`
and the final native gate. Foreign attribute metadata does not enable FFI.
This child installs identities and profiles; constant kernels, lowering,
scheduling, and runtime binding remain their ordered chunks. Native replacement
still depends on the exact declaration/signature, never spelling alone.

## Verification

[../tests/prelude.rs](../tests/prelude.rs) verifies source/body-free names,
types/units/literals/operators, implicit standard lookup, user shadowing,
existing-ID preservation, idempotence, binary/RON reload, forged/duplicate/
incomplete/stale bindings, ID exhaustion rollback, and old-schema rejection.
Affected artifact/codec/store/command/name suites also exercise the schema
transition; the readable artifact fixture is updated explicitly to schema `3`.

```sh
cargo test --test prelude --test intrinsics --locked
cargo fmt --all
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
```
