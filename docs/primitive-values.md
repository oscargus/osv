# Primitive Values and Time

Plan chunk `011`. APIs live in [../src/values/scalar.rs](../src/values/scalar.rs)
and [../src/values/time.rs](../src/values/time.rs), with shared `ValueError`
in [../src/values/mod.rs](../src/values/mod.rs). These are value primitives,
not a VHDL evaluator, event queue, or standard-package implementation.

## Scalars

`PrimitiveValue` distinguishes boolean, two-state bit, enumeration, integer,
real, physical, and time records through typed variants. Type/declaration
identity is separate from optimized storage. `BitValue::{Zero, One}` is not
the future nine-state logic type.

`IntegerValue(i64)` implements checked addition, subtraction, multiplication,
negation, and truncating division. Overflow, including negating `MIN` and
`MIN / -1`, and division by zero return distinct errors. `check_range` checks
subtype bounds without wrapping or saturating. Broader operator/conversion
families remain assigned to their semantic chunks, not approximated here.

`EnumerationValue` retains `TypeId` and ordinal position. Its constructor
checks a supplied literal cardinality. `validate(expected_type, cardinality)`
also checks restored values against the actual declaration; cardinality is
not redundantly stored in every value. Callers must do that contextual check
when they have the type table. Unrelated but similarly shaped enums are not
interchangeable.

`RealValue` stores exact `u64` `real_bits`. Constructors and both Serde decoders
reject NaN/infinity, preserve positive/negative zero, and retain finite maxima
and subnormals. `get()` reconstructs binary64 only when needed. Storage equality
is bit equality, not the future VHDL real equality operation; real arithmetic
and numerical/domain policy remain in `031` and the package chunks.

`PhysicalValue` retains `TypeId` and signed primary-unit count. `from_unit`
uses checked wide intermediate multiplication, rejects zero/out-of-range unit
scales, and checks the signed 64-bit result. It does not conflate distinct
physical types or implement the later physical arithmetic families.

## Integer Time

`TimeValue(i64)` is signed femtoseconds, including negative values and both
integer endpoints. `TimeUnit` covers fs, ps, ns, us, ms, sec, min, and hr using
exact integer scales; conversion overflow is explicit.

`TimeResolution` supports exactly 1 fs, 1 ps, 1 ns, 1 us, 1 ms, and 1 sec, with
1 fs default. `SimulationTime` holds nonnegative ticks plus its explicit
resolution. Constructors and Serde decoding require ticks at most
`TIME'HIGH / femtoseconds_per_tick`. `TimeValue::to_ticks` rejects negative
scheduled delays and any lost resolution; it never rounds or uses floating
point. `checked_advance` preserves resolution, accepts zero delay, and checks
the final simulation-time range. It does not assign delta cycles or scheduler
phases; those remain `025` onward.

## Persistence and Checks

Values use one Serde representation for Postcard and RON. Real bits, enum/type
IDs, unit counts, signed endpoints, ticks, and resolution remain lossless.
Finite-real and simulation-time invariants are checked during decoding, not
only by constructors. The existing artifact envelope is unchanged; future
typed IR integrates these values deliberately with contextual type/range
validation rather than accepting arbitrary enum positions.

[../tests/values.rs](../tests/values.rs) checks scalar overflow/ranges, invalid
enum positions/type identity, real-bit fidelity, physical scaling, time-unit
conversion, all supported resolutions, lost resolution, negative delays,
zero/final-time boundaries, and invalid decoding. The readable
[../tests/fixtures/values/primitives.ron](../tests/fixtures/values/primitives.ron)
uses the chunk `006` fixture and round-trip helpers for every primitive kind
and both encoding/conversion directions. It is explicitly a harness self-test:
there is no analyzer/simulator yet to run VHDL assertions, and no miniature
fixture parser or interpreted standard body is introduced as a substitute.

```sh
cargo test --test values --locked
cargo fmt --all
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
```
