# Initial Typed Scalar Expressions

Plan children `018a` and `018b`.
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
artifact format. Schema `4` embeds them in analysis object initializers;
see [analyzed-unit-data.md](analyzed-unit-data.md). Source unit integration is
implemented by `019b`. No parser
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

## Syntax Typing and Exact Literals

`analyze_expression` in [../src/analysis/typing.rs](../src/analysis/typing.rs)
consumes a local syntax node, validated artifact/type/scope context, optional
expected type, and known constant values. It returns an owned flat plan and
explicit static/deferred result, not a complete analyzed unit or CLI stage.
The supported subset includes integer and exact integer-valued time literals,
boolean literals through native declarations, simple names, parentheses,
selected integer/boolean/time operations, and zero-argument native time
observation. Operators resolve through the nearest visible declaration set
and exact native profiles; user-defined operators are never replaced by
same-spelled native kernels. IDs/types/reference metadata are attached during
analysis, not looked up from strings during later execution.

[../src/analysis/literals.rs](../src/analysis/literals.rs) decodes parser-validated
abstract-literal tokens. The pinned crate has no numeric semantic-value helper.
Underscore separators, decimal/based digits, and exponents use exact `BigUint`
arithmetic, with signed conversion only at contextual integer/physical range
checks. Negative minimum integers and signed time literals preserve the extra
negative endpoint. Physical scales come from resolved native TIME unit symbols,
not a spelling-based guessed unit table. Selected unary time negation appends
an intrinsic ID and changes the native manifest fingerprint deliberately.

Literal decoding has explicit resource limits (131,072 token bytes and exponent
at most 1,000,000) rather than truncation or huge unchecked allocation. Real and
fractional physical literals remain explicit unsupported features with coverage
`L003/L007/L008` and owner `031`. Generic-dependent/unknown constant values stay
not static; no signal/variable or time observation is folded speculatively.

`ExpressionOptions::fold_constants` controls retention versus simplification
of typed operations. Both paths use identical checked kernels and preserve
the same values/errors/deferred states. Mandatory signed literal conversion is
not a host overflow operation. Expected native subtype bounds are checked on
static results. Aggregate, selected/call/attribute, full overload, and later
scalar operation families are not silently enabled. Known legal but deferred
families return explicit unsupported diagnostics, rather than a false type-error
claim. Incompatible operands get primary/related spans and expected/actual
type notes. Parser token spans are checked and propagated, never guessed.

Expression syntax depth is bounded to 128 and owned nodes to 65,536. Scalar
parentheses reuse the parser's single unchosen element-association wrapper;
multiple/named aggregate associations are rejected, not reduced to one element.
Schema `4` embeds these plans in owned object initialization records;
source-driven unit integration is implemented by `019b`. No package body is loaded.

## Verification

[../tests/expressions.rs](../tests/expressions.rs) verifies exact-reference pure
folding versus `now`, flat-plan binary/RON conversion, equivalent folded and
unfolded plans, duplicate/dangling/forward/type errors, nonstatic signal and
unavailable constant behavior, checked integer/time/boolean results, overflow,
zero division, and bad argument types/arity. Plans are explicitly synthetic
foundation records.

[../tests/expression_syntax.rs](../tests/expression_syntax.rs) and
[../tests/literals.rs](../tests/literals.rs) additionally exercise real VHDL
expression fixtures, decimal/based exponent values, boolean/time operations,
missing names, range/overflow/zero-divisor errors, expected types, deferred
`now`, user-operator isolation, unsupported real/aggregate/family diagnostics,
and folded/unfolded/both-codec agreement. `ExpressionAnalysis` metadata names
the actual semantic API, distinct from parser probes and harness self-tests;
tests provide its typed declaration context, not a pretend whole-unit analyzer.
The independent color-free multi-operand/type golden is
[../tests/snapshots/diagnostics/expression-type-mismatch.ron](../tests/snapshots/diagnostics/expression-type-mismatch.ron).

```sh
cargo test --test expressions --locked
cargo fmt --all
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
```
