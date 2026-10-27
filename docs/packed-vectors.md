# Packed Binary Vectors

Plan chunk `013`. [../src/values/binary.rs](../src/values/binary.rs) provides
`PackedBinary` with independent width and `PackedStorage::{Small, Big}`.
Widths through 64 use one `u64`, including width zero; wider values use the
established `num_bigint::BigUint`. Leading zero width is metadata, not discarded
because a magnitude happens to fit a smaller host integer. Storage never uses
a per-bit vector for binary values.

## Construction and Interpretation

`from_u64` and `from_biguint` reject magnitudes that exceed the declared width.
`masked_from_u64` and `masked_from_biguint` explicitly select low-width masking
instead; callers must opt into that operation. Zero and full-word masks avoid
invalid host shifts. None of these methods implement VHDL assignment truncation
or a package conversion policy implicitly.

`MAX_VECTOR_BITS` is currently 32,000,000 bits (one million `u32` limbs), matching
the initial artifact collection budget. Wider construction returns an explicit
`VectorWidth` resource error; no bits are silently discarded. Aggregate input
budgets may reject multiple large values before that individual limit. This
is an implementation resource limit, not a claim that wider VHDL types are
syntactically illegal. `num-bigint` resolves to `0.4.8` via compatible `0.4`.

Bit zero is the rightmost logical element. `bit`/`set_bit` check this numeric
bit position; `logical_bit(range, index)` uses chunk `012`'s declared range to
map the leftmost element to the most significant stored bit, for either index
direction. It rejects range/width mismatches and invalid indices.

`unsigned_value` returns magnitude. `signed_value` interprets the same bits as
two's complement using the declared width, with exact arbitrary precision;
signedness is not a storage flag or a separate negative-number representation.
The empty bit string has magnitude/interpretation zero at this storage API.
These are mathematical storage interpretations, not a replacement for later
numeric package result-shape, null-input, warning, or conversion semantics.

## Stable Encoding

Both Serde codecs use the same named `PackedBinary` record:

```ron
PackedBinary(width: 128, storage: Big(limbs_le: [2, 0, 1]))
```

The example represents `2^64 + 2` with retained width 128. Wide magnitudes use
explicit least-significant-first `u32` limbs, not `BigUint`'s private host
representation or platform-size digits. Zero has an empty limb list; high
zero limbs are rejected. Decode also rejects a storage kind inconsistent with
width, excess high bits, excess limb count, and the width resource limit.
Small values encode `Small(u64)` with checked masking invariants. Packed fields
are private, so construction/decoding cannot publish an invalid combination.

Production artifact readers still supply the shared input/allocation limits
before deserializing collections. Standalone trusted-value codec tests do not
replace those guards. The artifact-envelope schema and type/index metadata
remain unchanged; integrating packed values into typed expressions is later
work. Nonbinary states are not approximated as integers and remain `014`.

## Checks

[../tests/binary_vectors.rs](../tests/binary_vectors.rs) covers widths 0, 1,
31, 32, 63, 64, 65, 128, 4096, and 131,072, leading zeros, invalid magnitudes,
sign-only/all-one negatives, both index directions, canonical limb decoding,
and the readable [../tests/fixtures/values/binary.ron](../tests/fixtures/values/binary.ron).
`proptest` `1.12.0` generates magnitudes around specialization boundaries and
wide values, checking every retained bit plus both codecs/conversion directions.
The fixture remains an explicit harness self-test rather than a claimed VHDL
simulation. No standard package body or new parser is introduced.

```sh
cargo test --test binary_vectors --locked
cargo fmt --all
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
```
