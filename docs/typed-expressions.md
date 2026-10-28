# Initial Typed Expression Foundation

Plan child `018a`. Syntax-driven typing/literal diagnostics remain `018b`.
Flat records live in
[../src/analysis/expressions.rs](../src/analysis/expressions.rs); checked
constant kernels live in [../src/analysis/constant.rs](../src/analysis/constant.rs).

## Owned Plans

`ExpressionPlan` contains an ordered vector of `TypedExpression` records plus
an explicit result `ExpressionId`. Each record retains concrete `TypeId`,
source `ByteSpan`, and a literal, resolved `SymbolId`, or exact intrinsic
reference with earlier operand expression IDs. It is not a recursive parser
tree; boxed intrinsic metadata is owned data, not a host function pointer.
All records support the same Postcard/RON schema.

`validate(&ArtifactEnvelope)` requires a valid artifact/type/source/native
context. It rejects duplicate expression IDs, missing result/types/symbols,
invalid source spans, forward/dangling operand references, literal/type
incompatibility, and mismatched intrinsic profiles/arguments/results/native
requirements. Subtype identity retains the native integer/time base while
actual static values still check subtype bounds. Only the initial boolean,
integer, and time expression subset is accepted; unsupported literal/type
families fail rather than being coerced or assigned arbitrary values.

The records are serializable foundation data, not a new unversioned CLI
artifact format. They are not yet fields in the current artifact envelope;
later analyzed-unit integration must change that schema deliberately. No parser
node, arena, closure, runtime stack, or host binding is stored.

## Checked Folding

`constant::fold(reference, arguments)` validates the exact trusted reference
and arity, respects registry folding policy, and returns `FoldResult::Value`
or explicit `NotYetStatic`. Time observation, report, and assert metadata are
never executed by folding. Supported pure integer arithmetic/comparisons,
boolean operations, and time addition/subtraction/comparisons use checked
primitive kernels; overflow, division by zero, wrong arity/type, unsupported
operation, and invalid identities return typed errors.

`ExpressionPlan::evaluate` is analysis-time partial evaluation over owned typed
records, not runtime parser interpretation. Missing known-constant values remain
not static. Supplied signal/variable values cannot turn a read into an analysis
constant. Constant, literal, and physical-unit candidates still validate their
resolved native types/ranges. Deferred operands propagate not-static results;
they do not become zero, false, a guessed generic, or an eager effect.

Later lowering/runtime can bind the resolved operations to specialized checked
value kernels without overload/name lookup. This foundation does not execute
a process, schedule events, fold arbitrary user functions, perform real/array
arithmetic, or introduce a standard-package body.

## Verification

[../tests/expressions.rs](../tests/expressions.rs) verifies exact-reference pure
folding versus `now`, flat-plan binary/RON conversion, equivalent folded and
unfolded plans, duplicate/dangling/forward/type errors, nonstatic signal and
unavailable constant behavior, checked integer/time/boolean results, overflow,
zero division, and bad argument types/arity. Plans are explicitly synthetic
until `018b` produces them from VHDL fixtures.

```sh
cargo test --test expressions --locked
cargo fmt --all
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
```
