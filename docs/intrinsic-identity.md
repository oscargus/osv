# Native Declaration Identity

Plan child `017a`; prelude installation remains `017b`. The registry in
[../src/intrinsics/mod.rs](../src/intrinsics/mod.rs) is metadata and validated
references, not an executing kernel, standard-package body, or blanket
standard-library coverage claim.

## Canonical Profiles

`NativeType` identifies standard declaration types independently of local
artifact `TypeId` allocation. Named profiles in
[../src/intrinsics/types.rs](../src/intrinsics/types.rs) describe eighteen
standard type/subtype declarations, exact literal ordering, scalar limits,
physical units, and array index/element types. These include scalar/time,
severity, character/string, constrained natural/positive/delay subtypes,
standard vector declarations, and file mode/status enums. Attribute metadata
also records `foreign : string`; it does not authorize foreign execution.

Profiles are cross-checked with the pinned standard declarations and the
chunk `002` inventory. Character has 256 Latin-1 positions, retaining named
controls, DEL/C128-C159, and character-valued literals as distinct typed data.
Time units are exact integer fs through hr. Integer/time limits follow the
explicit OSV decisions from chunk `003`, not the narrower reference's INTEGER
range or its differing negative TIME endpoint. Real bounds are exact bits.
Array/file declarations here are shapes/identities, not implementations of
their later operations or I/O signatures.

`Signature` uses typed operator, function, or statement identity, argument
types, and optional result. Report/assert are statement effects, not invented
VHDL functions exported from a standard package. Stable `IntrinsicId` variants
currently identify selected integer/boolean/time operations, report/assert,
and `now` metadata. The latter is impure and returns `delay_length` as declared.
Other standard operations remain explicitly owned by later chunks.

## Trusted References

`IntrinsicRegistry::standard()` supplies the compiled-in declaration set.
Its BLAKE3 fingerprint covers the registry-profile version (`1`), native ABI,
canonical type/attribute profiles, and operation metadata, encoded with
Postcard. It is an OSV native-declaration manifest identity, not a hash of a
copied VHDL body or a claim that the upstream abbreviated source alone defines
all standard semantics. Changes to required profiles invalidate old references.

`DeclarationIdentity` retains complete `std.standard` unit identity, declaration
fingerprint, ABI, and signature. `resolve` compares all of them after basic-name
normalization. A matching unqualified spelling, profile in `work`, stale hash,
wrong ABI, or changed argument/result cannot select a native replacement.
`IntrinsicReference` retains the stable ID plus that declaration identity;
`definition(reference)` also verifies that the ID belongs to the identity.
No host pointer, closure, runtime state, or AST is serialized.

Definitions retain purity, specialization, observable effects, and folding
policy separately. Selected pure scalar operations permit static arguments.
Report/assert and time observation are never folded. Future signal-observing,
stateful, or package operations must declare their actual effects/policies;
they must not infer foldability from a function name or a blanket pure flag.
Load-time kernel binding and execution belong to later lowering/runtime work.

## Verification

[../tests/intrinsics.rs](../tests/intrinsics.rs) checks every registered
reference through both codecs/conversion directions, ID/profile consistency,
user homonym rejection, stale fingerprints/ABIs, signature/package differences,
effect/folding metadata, case normalization, and audited type/literal/time
boundaries. It reuses the established round-trip helper. No package body,
reference simulator, filesystem lookup, or VHDL interpreter is involved.

```sh
cargo test --test intrinsics --locked
cargo fmt --all
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
```
