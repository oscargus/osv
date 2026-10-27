# Lossless Logic Storage

Plan chunk `014`. [../src/values/logic.rs](../src/values/logic.rs) defines
`LogicState::{U, X, Zero, One, Z, W, L, H, DontCare}` and `LogicVector`.
`symbol()` yields the corresponding `U X 0 1 Z W L H -` character.
`binary()` succeeds only for exact `Zero` and `One`; weak, unknown, high
impedance, and don't-care values are never silently normalized.

## Representation

`LogicStorage::Binary` reuses `PackedBinary`, including its small/wide threshold
and retained width. Nonbinary values use `States` with two four-bit state codes
per byte in logical left-to-right order, plus a private-state nonbinary count.
The first element occupies the first byte's low nibble; unused padding stays
zero. Public access to storage is immutable and vector fields are private.
Buffers use checked width conversion and fallible reservation, with the same
32,000,000-bit/state storage cap as packed vectors.

`from_states` chooses binary storage when every element is exactly binary.
`from_packed` preserves an existing packed value. `promote()` is lossless even
for binary/empty values, and `try_repack()` succeeds only when no nonbinary
state remains. `set_state` promotes on demand and repacks after the final
nonbinary element becomes exact `0` or `1`. Replacing a value with `L` or `H`
retains that exact weak state and prevents repacking.

`state`/`set_state` use left-to-right storage offsets; internally packed bit
zero still means the rightmost element. `logical_state` maps through an
independent declared `IndexRange`, retaining either direction and nonzero
origin. Invalid offsets and width/range mismatches fail explicitly.

`slice(start, length)` copies a checked contiguous **storage-position** range;
null copies are legal, overflow/out-of-range copies fail, and the new vector
may repack when its selected elements are exactly binary. This helper does
not implement VHDL slice subtype/direction/assignment rules from `033`.
Resolution functions, logic truth tables, numeric interpretation, and weak
normalization policies remain outside this storage layer.

## Persistence and Equality

Storage equality compares logical states, so a deliberately promoted binary
value equals its packed counterpart. Both codecs use one canonical schema:
`Binary(PackedBinary(...))` or `States([U, X, Zero, ...])`. Nonbinary values
therefore expose meaningful named states in RON, not opaque nibble bytes.
Pure binary values serialize identically even if explicitly promoted in
memory. Decode reconstructs checked storage and canonicalizes exact binary
state lists; unknown enum variants fail.

Wire state lists are subject to the same configurable collection limits as
other artifact data (one million entries by default), independently of the
larger individual storage cap. A resource limit rejects input rather than
changing state values. The shared envelope schema is unchanged; typed IR
integration remains later work. No standard-package body is read.

## Verification

[../tests/logic_vectors.rs](../tests/logic_vectors.rs) verifies all nine states,
four-bit compactness, small/wide boundaries, lossless promotion, weak-state
non-repacking, restoration to binary, null/copy/index checks, both directions,
canonical binary output, and named-state fixture parity. `proptest` compares
random state updates with a lossless reference vector and both codec conversion
directions. The fixture at
[../tests/fixtures/values/logic.ron](../tests/fixtures/values/logic.ron) remains an
explicit harness self-test until actual VHDL analysis/simulation is available.

```sh
cargo test --test logic_vectors --locked
cargo fmt --all
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
```
