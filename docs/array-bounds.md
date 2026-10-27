# Array Bounds and Index Mapping

Plan chunk `012`. [../src/values/bounds.rs](../src/values/bounds.rs) contains
storage-independent metadata and checked logical/storage index conversion.
It does not allocate element values or implement array expressions.

`IndexRange` retains signed `left`/`right` endpoints and explicit
`Direction::{Ascending, Descending}`. Endpoints are inclusive. `length()` checks
for a null range before wide arithmetic; a legal null range has zero length
without losing its original endpoints/direction. Non-null lengths use `u64`;
the full signed-64-bit index domain contains `2^64` elements and fails length
conversion explicitly instead of wrapping. This is a storage-count limit, not
an assertion that the VHDL bounds syntax is illegal.

`offset(index)` maps the declared leftmost element to storage position zero,
regardless of index origin or direction. `index_at(offset)` is the inverse.
Neither assumes a zero-based or descending vector. Both check bounds, reject
null-range indexing, and use checked/wide arithmetic for extreme endpoints.

`RangeConstraint` distinguishes unconstrained metadata from a concrete
`IndexRange`. An `ArrayDimension` retains index `TypeId` and constraint; an
`ArrayShape` retains all dimensions without embedding element storage. Full
index-type legality remains a contextual type-table check in later analysis.
Unconstrained metadata serializes losslessly but cannot masquerade as an empty
or zero-based allocated array.

`ArrayShape::element_count`, `storage_offset`, and `indices_at` require one or
more fully constrained dimensions. The rightmost dimension varies fastest.
Products and offsets are checked, and index arity must match shape arity.
A constrained shape with any null dimension has zero total elements, even
when another range would exceed the non-null length limit; no storage or
arbitrary index value is invented. Every attempted access to an empty shape
fails. Nonempty shapes with unrepresentable products fail explicitly.

Common `ValueError` variants retain bad logical indices/ranges, bad storage
offsets/lengths, dimensional mismatch, or unconstrained bounds. The descriptor
can retain metadata that later resource/type checks reject; serialization
alone is not evidence that a huge or unconstrained array can be allocated.

The single Serde schema retains index types, endpoints, direction, constraints,
and dimension order in both Postcard and RON. It does not derive shape from
packed integer width or conflate index direction with numeric signedness.
No artifact-envelope schema or existing primitive contract changes.

[../tests/bounds.rs](../tests/bounds.rs) checks positive/negative nonzero
indices, both directions, inverse mapping, null/extreme/singleton ranges,
out-of-bounds indices, huge lengths/products, rightmost-fast multidimensional
mapping, unconstrained/zero-dimensional errors, and both-codec fixture parity.
The readable [../tests/fixtures/values/shape.ron](../tests/fixtures/values/shape.ron)
is an explicitly synthetic harness self-test until actual array analysis is
implemented. Packed storage, logic states, slicing, and assignment semantics
remain in their later chunks.

```sh
cargo test --test bounds --locked
cargo fmt --all
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
```
