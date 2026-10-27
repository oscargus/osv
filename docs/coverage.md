# VHDL 2008 Coverage Inventory

This document and [native-declarations.md](native-declarations.md) implement chunk `002` of [../PLAN.md](../PLAN.md). The scope follows [../project.md](../project.md), including all IEEE 1076-2008 packages, all three `std` packages, VCD/FST output, and GitHub Actions CI.

This is a target inventory, not a simulator support claim. Only the parser/source probes and inventory tooling exist. All implementation owners and fixture IDs below are planned unless a handoff explicitly records otherwise. No standard package body is loaded or interpreted by the inventory tool.

## Inventory Contract

- The generated report retains every explicit declaration profile from the selected references, including separate overloads, parameter names/classes/modes, results, defaults, aliases, constraints, constants, policies, imports, and context declarations. Empty compatibility packages receive a unit row.
- Identical token profiles merge their provenance links; source variants remain separate. Matching counts do not prove semantic conformance. The report is not a request to publish the union of incompatible source variants.
- Each report row has a proposed owner chunk, family/scope, stage eligibility, source line links, and a reserved `native.<unit>.<number>` test ID. IDs identify audit rows within these pinned inputs, not executable intrinsic IDs or a stable artifact ABI. Native dispatch/artifact IDs must be typed and declaration-based, not parsed from these strings.
- Alias rows describe declarations; their callable behavior and purity follow their resolved targets. A report's `A/E/R` eligibility does not authorize constant folding an impure, signal-dependent, or otherwise nonstatic call.
- Every required row needs positive, negative/domain, and boundary cases where applicable. Add actual VHDL fixture files, preferably with assertions, as implementations land; do not make a registry-presence test stand in for semantic coverage. Reserve `.ok`, `.err`, and `.edge` variants of each test ID.
- All future artifact-based tests run binary and RON inputs and compare observations. VCD/FST are simulation outputs, not additional encodings of analyzed/elaborated artifacts.

## Pinned Sources

| Source | Revision | Role |
| --- | --- | --- |
| [Rust-HDL](https://github.com/VHDL-LS/rust_hdl/tree/dc8c090f7aeb9a95149eadc5e8b48f170382a8d4/vhdl_libraries) | `dc8c090f7aeb9a95149eadc5e8b48f170382a8d4` | Requested Git `master` parser dependency and bundled declarations; retained from chunk 001. |
| [GHDL](https://github.com/ghdl/ghdl/tree/a977191e75d345ee30a936279f425265e52b37b3/libraries) | `a977191e75d345ee30a936279f425265e52b37b3` | Independent IEEE declarations, revision-selected `std` interfaces, and compiler-provided operations. |
| [NVC](https://github.com/nickg/nvc/tree/8630448e4a25aa4bd74066b94eba08063597a3ca/lib) | `8630448e4a25aa4bd74066b94eba08063597a3ca` | All `std.08` interfaces and IEEE references from `ieee.08`, with `math_real`/`math_complex` inherited from `ieee`. |

GHDL and NVC are source references here; neither simulator has been built or executed by this chunk. These revision-selected implementations supply an auditable 2008 reference, not a replacement for the language standard or proof of conformance. Implementation chunks must check semantic rules and disputed cases against IEEE 1076-2008 rather than blindly copying one simulator's behavior.

The audit checks each declaration file byte-for-byte against its pinned Git blob and verifies each checkout's `HEAD`. It does not reject unrelated changes elsewhere in a Cargo-managed checkout; it does not revert them. GHDL's `START/END-V08`, `START/END-V19`, and `--V87` template markers are selected for 2008 while preserving byte offsets and line numbers. Unknown/unbalanced block markers fail the audit.

Licensing/provenance remains per source. Rust-HDL's parser has MPL-2.0 notices; IEEE declaration files and NVC's inspected standard declarations have Apache-2.0 notices; GHDL's `std` sources carry GPL notices. Preserve original attribution and review licensing before vendoring. This chunk references declarations and records interface profiles; it does not copy or translate executable package bodies.

### Reconciled Differences

| ID | Observation | Required Decision / Owner |
| --- | --- | --- |
| D01 | Rust-HDL's `std.standard` source omits many compiler-provided operations. NVC's source omits the explicit bit/boolean edge declarations present in Rust-HDL. | Include the implicit templates below plus all four standard bit/boolean edge overloads. Owners `035`, `040`, `057`. |
| D02 | Rust-HDL and NVC use different lower bounds for `INTEGER` and `TIME`. | Retain both source variants as implementation-defined limits; choose OSV's limits in `003`, not by merging declarations. |
| D03 | GHDL's `std.textio` declaration omits `WRITE(L, REAL, FORMAT : STRING)`, but NVC's `std.08` and Rust-HDL include it. | It is required 2008 coverage, owner `107`; do not misclassify it as 2019 merely because one reference omits it. |
| D04 | Rust-HDL's `std.textio` explicitly includes `FILE_REWIND`, `FILE_MODE`, and `FILE_SIZE`. They are absent from both selected 2008 references and appear with later file-position facilities. | Exclude these from OSV's 2008 prelude, diagnose their use, owner `015`. Do not remove ordinary 2008 file operations or `FLUSH`. |
| D05 | GHDL's `std.env` template includes assert-count/clear routines inside a V19 block. NVC has additional environment/reflection facilities under `std.19`. | Exclude later interfaces; 2008 `std.env` has both `STOP` overloads, both `FINISH` overloads, and `RESOLUTION_LIMIT`. Owner `059`. |
| D06 | NVC adds `never_waits` metadata and an internal `FOREIGN` binding for `NOW`. | Record as reference annotations, not required public native API or authorization to implement foreign integration. Owner `110` for applicable language attribute checks. |
| D07 | `numeric_std` has copyright-string variants, and NVC spells `U_SIGNED`/`U_UNSIGNED` as aliases where Rust-HDL/GHDL use subtypes. | Preserve source variants. Bind the 2008 public names to the same underlying type/subtype constraints and test identity, overload selection, and resolution in `063`; do not create competing declared types. Copyright constants are provenance, not numerical behavior. |
| D08 | The 2008 `std_logic_textio` package is empty; I/O declarations reside in `std_logic_1164`. | Retain the compatibility unit and analyze its visibility correctly in `103`; no legacy body or duplicate overloads. |
| D09 | The parser documents incomplete non-2008 revision handling and no PSL support. | Explicit 2008 selection plus revision guards remain mandatory. Track PSL as a parser gap below, not silently supported or approved for removal. Owners `015`, `119`. |

The generator rejects unassigned function names and unreconciled Rust-HDL/GHDL functional IEEE profiles. It retains NVC variants instead of normalizing away declaration-kind differences. This comparison checks profiles, not function semantics. No numerical fallback to a reference subprocess or standard VHDL body is permitted.

## Packages, Dependencies, and Owners

`std.standard` is implicitly visible throughout. Explicit imports below are declaration dependencies; shared native kernels are listed separately where they are not declaration imports. `work` inside an IEEE package denotes its current library, `ieee`, not the user's `work` library.

| Unit | Declaration Dependencies / Native Reuse | Native Owners / Final Gate |
| --- | --- | --- |
| `std.standard` | Builtin scalar/physical/array/file-status declarations and language-generated operations. | `017`, `031`-`040`, `057`, `105`-`106`; final `122`. |
| `std.env` | Standard integer/status/time types and configured time resolution. | Every environment signature `059`; final `122`. |
| `std.textio` | Standard strings/scalars/time, access `LINE`, file `TEXT`, enum `SIDE`, subtype `WIDTH`, standard I/O objects. | Access/file support `105`-`106`; I/O `107`-`108`; final `122`. |
| `ieee.std_logic_1164` | `std.textio` declarations for its I/O signatures; standard bit/boolean types. | Resolution `052`, cores `061`-`062`, I/O `108`; final `122`. |
| `ieee.numeric_std` | `std_logic_1164`, `std.textio`; independent signed/unsigned declaration identities. | Cores `063`-`068`, I/O `108`; final `122`. |
| `ieee.math_real` | Standard real/integer types; mutable seeds for `UNIFORM`. | `069`-`071`; final `122`. |
| `ieee.math_complex` | Current-library `math_real`, standard real constraints and record values. | `081`-`087`; final `122`. |
| `ieee.fixed_float_types` | Standard enum declarations for fixed/floating policies. | `072`; final `122`. |
| `ieee.fixed_generic_pkg` | `std_logic_1164`, `numeric_std`, `fixed_float_types`, `std.textio`. | Generic machinery `047`, policies `072`, core `073`-`076`, I/O `108`; final `122`. |
| `ieee.fixed_pkg` | Instance of `fixed_generic_pkg` with explicit policy actuals. | `047`, `072`-`076`, `108`; final `122`. |
| `ieee.float_generic_pkg` | `std_logic_1164`, `numeric_std`, `fixed_float_types`, `std.textio`; generic formal package instance of `fixed_generic_pkg`. | `047`, `072`, `077`-`080`, I/O `108`; final `122`. |
| `ieee.float_pkg` | Instance of `float_generic_pkg`, passing `ieee.fixed_pkg` and policy actuals. | `047`, `072`, `077`-`080`, `108`; final `122`. |
| `ieee.numeric_bit` | Standard bit arrays and `std.textio`; reuse binary storage/arithmetic kernels, not logic type identities. | Cores `088`-`093`, I/O `108`; final `122`. |
| `ieee.numeric_bit_unsigned` | Standard `BIT_VECTOR`; native adapters reuse `numeric_bit` kernels. | `094`-`097`; final `122`. |
| `ieee.numeric_std_unsigned` | `std_logic_1164`; native adapters reuse `numeric_std` kernels. | `098`-`102`; final `122`. |
| `ieee.std_logic_textio` | Empty compatibility unit; native I/O remains owned by `std_logic_1164`. | `103`; I/O checks `108`; final `122`. |
| `ieee.ieee_bit_context` | Context imports `std`/`ieee` visibility and the standard bit-numeric context dependencies. Exact context text is in the generated report. | Context analysis `039`, registration `103`; final `122`. |
| `ieee.ieee_std_context` | Context imports `std`/`ieee` visibility and the standard logic/numeric context dependencies. Exact context text is in the generated report. | Context analysis `039`, registration `103`; final `122`. |

Native bootstrap must register trusted package/type/declaration identities, resolve their declaration imports, attach versioned native bindings to exact signatures, and retain source/provenance and policy metadata. Generic packages specialize through the ordinary semantic machinery. User packages with the same unqualified names must not acquire native replacements. Executable pointers, open files, RNG state, and mutable process objects are not serialized into analysis/elaboration artifacts.

### Generic Policies

| Package / Parameter | Declared Default / Domain | Planned Tests |
| --- | --- | --- |
| `fixed_round_style` | `fixed_round_style_type`: `fixed_round`, `fixed_truncate`; default `fixed_round`. | `policy.fixed.round`: ties, guard bits, truncation, generic association. |
| `fixed_overflow_style` | `fixed_overflow_style_type`: `fixed_saturate`, `fixed_wrap`; default `fixed_saturate`. | `policy.fixed.overflow`: signed/unsigned extrema and resize. |
| `fixed_guard_bits` | `NATURAL := 3`. | `policy.fixed.guard`: zero/default/nondefault and invalid negative actual. |
| Fixed `no_warning` | `BOOLEAN := false`. | `policy.fixed.warning`: identical values with the specified diagnostic difference. |
| `float_exponent_width`, `float_fraction_width` | `NATURAL := 8`, `NATURAL := 23`; format validity has additional package rules. | `policy.float.format`: non-host widths, format boundaries, invalid combinations. |
| `float_round_style` | `round_type`: `round_nearest`, `round_inf`, `round_neginf`, `round_zero`; default `round_nearest`. | `policy.float.round`: ties, signs, all rounding policies. |
| `float_denormalize` | `BOOLEAN := true`. | `policy.float.denormal`: on/off underflow and subnormals. |
| `float_check_error` | `BOOLEAN := true`. | `policy.float.error`: exceptional operands/results and policy behavior. |
| `float_guard_bits`, floating `no_warning` | `NATURAL := 3`, `BOOLEAN := false`. | `policy.float.guard-warning`: exact/inexact results and warning behavior. |
| Floating formal `fixed_pkg` | Formal instance of `IEEE.fixed_generic_pkg` with `generic map (<>)`; the default `float_pkg` instance passes `IEEE.fixed_pkg`. | `policy.float.fixed-actual`: matching/mismatching formal packages and distinct policies. |

Owners are `047`, `072` and the relevant fixed/floating operation chunk. The complete generic clauses, operation-specific defaults, and default instance maps are preserved in the generated report. Do not confuse an operation default referencing a package generic with a hardcoded constant. Policy actuals participate in specialization and dependency/artifact identity in both encodings.

## Implicit Predefined Operations

Package declaration scans cannot enumerate operations generated for every user-declared type. These templates supplement every explicit row. Instantiate the applicable templates for each declared type, respecting language restrictions; do not inject all operators into all types. `T`, `A`, `E`, and `F` below are schematic types, not new package declarations.

| ID / Reserved Test | Required Family / Signature Templates | Owner | Checks |
| --- | --- | --- | --- |
| `implicit.scalar.compare` | Equality/inequality and ordering for eligible scalar `T`: `(T,T) -> BOOLEAN`; scalar `MINIMUM`/`MAXIMUM` where predefined. | `034`, `040` | Distinct declared types, enum order, physical units, null subtype bounds. |
| `implicit.numeric.arithmetic` | Integer/real/physical unary and binary arithmetic, `abs`, exponentiation, integer `mod`/`rem`, and applicable physical multiplication/division signatures. | `031`, `040` | Universal literals, result types, signs, zero divisors, overflow, conversions. |
| `implicit.logic.predefined` | Boolean/bit logical operations, eligible array logical/reduction, condition and matching/shift families defined by the language. | `040`, `111` | Type-appropriate operator sets, shifts/counts, matching versus ordinary equality; no accidental overloads. |
| `implicit.composite.equal` | Equality/inequality for eligible array/record types; applicable one-dimensional lexicographic ordering and extrema. | `032`, `040` | Nested components, lengths, null arrays, direction, prohibited type combinations. |
| `implicit.array.concat` | Concatenation overloads `(A,A)`, `(A,E)`, `(E,A)`, `(E,E) -> A` for eligible one-dimensional arrays. | `033`, `040` | Result bounds, null operands, element/array ambiguity, direction changes. |
| `implicit.scalar.attribute` | Scalar type/subtype attributes: bounds/ranges/base, position/value/image, predecessor/successor/leftof/rightof as applicable. | `035` | Enum/real/physical cases, out-of-range values, invalid attribute prefix. |
| `implicit.array.attribute` | Array bounds/length/ascending/range/reverse-range, including dimension selection and applicable element/index attributes. | `012`, `032`, `035` | Nonzero indices, multidimensional and unconstrained arrays, invalid dimensions, nulls. |
| `implicit.string.convert` | Required `TO_STRING` scalar/eligible-array operations, additional real digits/format and physical/time unit overloads, bit-vector octal/hex conversion. | `035`, `040` | Width/format/unit rules, empty values, bounds, signedness; do not omit operations absent from the bundled `standard` file. |
| `implicit.signal.edge` | `RISING_EDGE`/`FALLING_EDGE` overloads for `BOOLEAN` and `BIT`; package-specific logic/bit-numeric helpers retain their own signatures. | `057`, `062`, `091` | Initial state, activity without change, transition history, resolved values. |
| `implicit.signal.observe` | `event`, `active`, `last_event`, `last_active`, `last_value`, `driving`, `driving_value`. | `057` | Per-signal versus per-driver observations and same-time/delta activity. |
| `implicit.signal.generated` | Signal-valued `delayed`, `stable`, `quiet`, `transaction`, including applicable time defaults. | `058` | Initialization, rescheduling, sensitivity, unchanged-value transactions. |
| `implicit.access` | Access equality/inequality, null, allocation/dereference, and `DEALLOCATE(P : inout T)` for eligible access types. | `105` | Null/stale access, aliasing, composites, lifetime; never serialize host pointers. |
| `implicit.file.open` | Per-file-type `FILE_OPEN` overloads with/without status, default `READ_MODE`, and `FILE_CLOSE`. | `106` | Modes, status, missing files, repeated opens, external-name policy. |
| `implicit.file.transfer` | Per-file-type `READ`, `WRITE`, `ENDFILE`, applicable unconstrained-array read/length behavior, and 2008 `FLUSH`. | `106` | EOF, element constraints, modes, cleanup; exclude later seek/tell/size APIs. |
| `implicit.name.attribute` | Hierarchical/simple/path/instance-name attributes and user attribute declarations/specifications. | `110` | Generated instances, aliases, source paths, non-ASCII identifiers. |
| `implicit.runtime.standard` | `NOW`, assertion/report severity behavior, default initialization, and range/index checks surviving static evaluation. | `017`, `020`, `025`, `028`, `032` | No impure folding; time-resolution and runtime source context; type-correct defaults. |

Independent compiler references include [GHDL's standard package builder](https://github.com/ghdl/ghdl/blob/a977191e75d345ee30a936279f425265e52b37b3/src/vhdl/vhdl-std_package.adb), [implicit file-operation construction](https://github.com/ghdl/ghdl/blob/a977191e75d345ee30a936279f425265e52b37b3/src/vhdl/vhdl-sem_utils.adb), and NVC's revision-specific declarations. `FLUSH` is gated for VHDL 2008 in the reference; its presence must not be confused with the excluded later file-position helpers.

## Language Feature Matrix

`A`, `E`, and `R` cells describe required analysis, elaboration, and runtime work with owner chunks. `n/a` means the feature has no independent work at that stage, not that it is unsupported. Every row below is planned; only the narrow syntax evidence from chunk `001` has run. The IDs in the last columns reserve fixture/diagnostic cases, not existing tests or implemented diagnostic codes. Append `.ok`, `.err`, and `.edge` where applicable.

| ID / Feature | Analysis | Elaboration | Runtime | Diagnostics | Planned Tests |
| --- | --- | --- | --- | --- | --- |
| L001 Source bytes, Latin-1, comments, positions | `005`,`015`: preserve bytes/spans | `048`: retain source tables | `028`: source fallback | `diag.source` | `lang.source` |
| L002 2008-only syntax and recovery | `015`: revision guards, typed syntax errors | n/a | n/a | `diag.revision` | `lang.revision` |
| L003 Basic/extended identifiers and literals | `015`,`016`,`018`: lexical identity and literal typing | `041`: deferred bounds | `027`: checked values | `diag.literal` | `lang.literal` |
| L004 Separate units/files/libraries | `016`,`021`: dependency order and unit identity | `024`,`048`: exact closure | `028`: source-free loading | `diag.unit` | `lang.separate-files` |
| L005 Primary/secondary units | `019`,`039`,`045`: associate bodies/architectures/configurations | `022`,`045`: selection | n/a | `diag.secondary` | `lang.primary-secondary` |
| L006 Context/use clauses and visibility | `016`,`034`,`039`: imports, homographs, overloads | `048`: exact dependencies | n/a | `diag.visibility` | `lang.visibility` |
| L007 Scalar enumerations/integer/real | `018`,`031`,`032`: identity and constraints | `041`: finalize static bounds | `011`,`031`: checked kernels | `diag.scalar` | `lang.scalar` |
| L008 Physical types and time | `031`,`032`: unit definitions and types | `041`: resolution checks | `011`,`025`: integer ticks | `diag.physical` | `lang.physical` |
| L009 Subtypes and constrained/unconstrained types | `032`,`035`: legality and staticness | `041`,`049`: concrete constraints | `033`: dynamic checks | `diag.constraint` | `lang.constraint` |
| L010 Arrays, dimensions, direction, null ranges | `012`,`032`,`033`: shape/index types | `041`,`042`: final bounds | `033`: indexing/storage | `diag.array` | `lang.array` |
| L011 Records and nested/unconstrained composites | `032`,`033`: fields/element constraints | `041`: concrete layout | `033`: values/copies | `diag.record` | `lang.record` |
| L012 Access/incomplete types | `032`,`105`: completion and designated types | `105`: allocation plans | `105`: handles/lifetimes | `diag.access` | `lang.access` |
| L013 File types/objects and implicit operations | `106`: classes, modes, constraints | `106`: open plans | `106`: checked I/O | `diag.file` | `lang.file` |
| L014 Protected types/shared variables | `109`: declarations, bodies, restrictions | `109`: per-instance state | `109`: atomic methods | `diag.protected` | `lang.protected` |
| L015 Constants, variables, signals, defaults | `019`,`032`,`039`: object rules | `022`,`041`: initialization | `026`,`027`: distinct assignment semantics | `diag.object` | `lang.object` |
| L016 Aliases, attributes, external names | `110`: reference/type rules | `110`: hierarchy binding | `110`: object access | `diag.reference` | `lang.reference` |
| L017 Resolution/subtype indications | `032`,`053`: function signatures and restrictions | `042`,`053`: driver/type layout | `052`,`053`: batch resolution | `diag.resolution` | `lang.resolution` |
| L018 Overloads, expected types, qualification | `018`,`034`: universal literals/candidates | `041`: generic specialization | `023`: no overload searches | `diag.overload` | `lang.overload` |
| L019 Conversions and range checks | `031`,`034`,`035`: legal conversion families | `049`: deferred checks | `031`,`033`: value checks | `diag.conversion` | `lang.conversion` |
| L020 Aggregates, choices, others, concatenation | `033`: type/context and completeness | `041`: constrained shapes | `033`: composite construction | `diag.aggregate` | `lang.aggregate` |
| L021 Indexed/selected/sliced names | `033`,`034`: target/type legality | `042`: port/view constraints | `033`: dynamic indexing | `diag.index` | `lang.index` |
| L022 Operators and matching/condition conversion | `034`,`040`,`111`: legal overload domains | `049`: specialization | `040`,`111`: exact operations | `diag.operator` | `lang.operator` |
| L023 Local/global staticness and partial evaluation | `018`,`035`: purity/staticness | `041`,`049`: generic-dependent checks | `027`: retain required checks | `diag.staticness` | `lang.staticness` |
| L024 Functions, purity, parameters, recursion | `037`: scopes/results/restrictions | `046`,`049`: specialization | `037`: call frames | `diag.function` | `lang.function` |
| L025 Procedures, classes/modes, suspension | `038`: parameter and wait restrictions | `038`: frame layout | `038`: resume nested calls | `diag.procedure` | `lang.procedure` |
| L026 Generic types/subprogram interfaces | `046`: matching/defaults | `046`: concrete instantiation | `046`: bound calls | `diag.generic-subprogram` | `lang.generic-subprogram` |
| L027 Packages, bodies, deferred constants | `039`: visibility/completion/dependencies | `041`,`048`: initialization/closure | `037`,`038`: user bodies | `diag.package` | `lang.package` |
| L028 Generic packages and formal packages | `047`: interface legality | `047`,`049`: type/policy identities | `047`: specialized calls/state | `diag.generic-package` | `lang.generic-package` |
| L029 Variable/signal assignments | `020`,`033`,`111`: targets and expressions | `023`: typed lowering | `026`,`050`,`051`: update semantics | `diag.assignment` | `lang.assignment` |
| L030 If/case, matching and conditional forms | `036`,`111`: choices/conditions | `049`: legal simplification | `036`,`111`: branches | `diag.branch` | `lang.branch` |
| L031 Loops, next, exit, returns, null | `036`,`037`: control targets | `023`,`036`: continuation lowering | `036`,`038`: state and resumption | `diag.control` | `lang.control` |
| L032 Assert/report and severity | `020`: types/severity | `049`: preserve observable operations | `028`,`059`: diagnostics/status | `diag.assertion` | `lang.assertion` |
| L033 Wait on/until/for and combinations | `020`,`055`: legality/sensitivity | `023`: continuation state | `055`: events/timeouts/cancellation | `diag.wait` | `lang.wait` |
| L034 Concurrent assignments/assertions/calls | `019`,`119`: semantic lowering audit | `023`,`042`: process/driver lowering | `026`,`038`,`050`: execution | `diag.concurrent` | `lang.concurrent` |
| L035 Entity generics and ports | `019`,`034`: interfaces/associations | `041`,`042`: actuals/defaults/layout | `042`: propagation | `diag.port-generic` | `lang.port-generic` |
| L036 Direct instances, hierarchy, blocks | `042`,`043`: scopes/interfaces | `042`,`043`: allocate/connect | `042`: independent state | `diag.instance` | `lang.instance` |
| L037 For/if/case generate | `043`: declarations/static legality | `043`: selected branches/iterations | `023`: lowered behavior only | `diag.generate` | `lang.generate` |
| L038 Components/default binding | `044`: component interfaces | `044`: library/binding rules | n/a | `diag.binding` | `lang.binding` |
| L039 Configurations/specifications | `045`: declaration/association rules | `045`: recursive explicit binding | n/a | `diag.configuration` | `lang.configuration` |
| L040 Initialization and default values | `019`,`032`,`039`: type-correct defaults | `022`,`041`: per-instance values | `025`,`026`,`056`: initial cycle | `diag.initialization` | `lang.initialization` |
| L041 Time/delta/phase ordering | `020`: delay types | `024`: concrete time policy | `025`,`056`: simulation cycle | `diag.scheduler` | `lang.scheduler` |
| L042 Transport waveforms/transaction replacement | `050`: expression/ordering rules | `023`: assignment plans | `050`: driver projection/cancellation | `diag.transport` | `lang.transport` |
| L043 Inertial delays/rejection limits | `051`: reject/delay legality | `023`: assignment plans | `051`: pulse/transaction rules | `diag.inertial` | `lang.inertial` |
| L044 Multiple drivers and composite resolution | `052`,`053`: resolution legality | `042`,`053`: driver graph | `052`,`053`: effective values | `diag.driver` | `lang.driver` |
| L045 Guarded blocks/bus/register/disconnection | `054`: declarations and guards | `054`: driver policy | `054`: disconnection scheduling | `diag.guard` | `lang.guard` |
| L046 Sensitized processes and process(all) | `055`: implicit sensitivity/wait restrictions | `023`: bound signal registrations | `055`: activation deduplication | `diag.sensitivity` | `lang.sensitivity` |
| L047 Postponed processes | `056`: restrictions | `023`: phase metadata | `056`: correct ordering | `diag.postponed` | `lang.postponed` |
| L048 Signal/history/driving attributes | `057`: prefix/type restrictions | `057`: history registration | `057`: event/activity distinction | `diag.signal-attribute` | `lang.signal-attribute` |
| L049 Implicit signal-valued attributes | `058`: arguments/types | `058`: implicit signal instances | `058`: scheduled changes | `diag.implicit-signal` | `lang.implicit-signal` |
| L050 Force/release | `112`: target/mode legality | `112`: driver integration | `112`: effective/driving semantics | `diag.force` | `lang.force` |
| L051 Standard/native declaration identity | `017`,`034`: exact trusted signatures | `047`,`049`: policy specialization | `061`-`108`: native kernels | `diag.intrinsic` | `lang.intrinsic` |
| L052 IEEE contexts and compatibility package | `039`,`103`: units/imports | `048`,`103`: exact closure | n/a | `diag.ieee-context` | `lang.ieee-context` |
| L053 Integer/big/nonbinary storage equivalence | `013`,`014`: typed value metadata | `041`: concrete widths | `013`,`014`: equivalent representations | `diag.value-width` | `lang.value-width` |
| L054 Diagnostic labels and portable snapshots | `005`,`120`: structured context | `049`,`120`: related instance sites | `028`,`120`: call/value context | `diag.rendering` | `lang.diagnostic` |
| L055 Analysis artifact per primary/secondary links | `007`,`010`,`021`: persist identity | `024`,`048`: exact dependency closure | `028`: validated loading | `diag.analysis-artifact` | `lang.analysis-artifact` |
| L056 Binary/RON schema, conversion, inspection | `007`-`009`: shared logical model | `024`,`048`: stage artifacts | `028`,`121`: bounded validation | `diag.codec` | `lang.codec` |
| L057 Cache invalidation and compatibility | `010`: dependency fingerprints | `049`: generic/native versions | `121`: reject stale/incompatible | `diag.compatibility` | `lang.compatibility` |
| L058 Linked/bundled source-free designs | `021`: stable references | `048`: deduplication/source data | `028`,`048`: fresh-process execution | `diag.bundle` | `lang.bundle` |
| L059 Stop/finish, limits, resolution_limit | `059`: exact environment overloads | `024`: time/resource policy | `059`: explicit termination/status | `diag.lifecycle` | `lang.lifecycle` |
| L060 Text/logic/numeric I/O | `106`-`108`: modes/overload profiles | `106`: external-file plans | `107`,`108`: native transfers | `diag.textio` | `lang.textio` |
| L061 VCD/FST signal registry and type mappings | `113`: typed observation metadata, not language analysis | `113`: hierarchical registration | `113`,`117`: initial/effective changes | `diag.waveform-type` | `waveform.mapping` |
| L062 VCD writer | n/a | `113`: selected metadata | `114`: checked streamed output | `diag.vcd` | `waveform.vcd` |
| L063 FST via fst_writer | n/a | `113`: selected metadata | `115`: checked FST output | `diag.fst` | `waveform.fst` |
| L064 Waveform CLI/selection/lifecycle | n/a | `116`: artifact/object selection | `116`: optional/both writers, cleanup | `diag.waveform-output` | `waveform.lifecycle` |
| L065 Waveform readback/parity | n/a | `118`: both artifact encodings | `118`: independent decode comparisons | `diag.waveform-reader` | `waveform.parity` |
| L066 GitHub Actions Rust/platform gates | `004`: workflow/toolchain design | n/a | `004`,`030`,`118`: required jobs | `diag.ci-prerequisite` | `ci.github` |
| L067 Embedded PSL verification | Parser gap; `015` must reject explicitly and track needed parser support | `119`: resolve implementation children before claiming conformance | Not implemented; no no-op assertions | `diag.psl-unsupported` | `lang.psl` |
| L068 Foreign attribute versus external execution | `110`: recognize/check applicable attribute syntax | External binding needs explicit scope approval | Foreign/mixed-language execution not committed | `diag.foreign-unsupported` | `lang.foreign` |

Concurrent calls/assertions/assignments must lower through ordinary process/control/type machinery rather than a second interpreter.

### Unsupported Versus Implementation-Defined

| ID | Category | Rule / Owner |
| --- | --- | --- |
| S01 | Later revisions | Reject VHDL-2019-only syntax, views, reflection, new environment/file-position APIs, and later overloads unless separately approved. Revision tests in `015`, reconciliation `119`. |
| S02 | Parser gap, not an approved language omission | Embedded PSL is not implemented by the pinned parser. Diagnose explicitly now; add parser/implementation children or obtain a scope decision before any full-language conformance claim. Owners `015`,`119`. |
| S03 | Separate standards/extensions | Synopsys compatibility, VITAL, `ieee_proposed`, AMS, mixed-language/foreign execution, and GUI/distributed simulation are not implied by IEEE 1076-2008 package coverage. Unknown libraries/declarations must produce useful errors, not fallback behavior. |
| S04 | Resource and representation limits | Integer/physical/time ranges, tick resolution, real precision, allocation/index/format limits, recursion and event/delta budgets are implementation decisions, not permission to silently reject arbitrary valid constructs. Decide/document in `003` and enforce at the appropriate stage. |
| S05 | Numerical approximation | Real/complex transcendental tolerances and implementation limits need documented reference/error policies in `069`-`071`, `081`-`087`. Fixed/floating package bit/rounding semantics remain exact to their specified formats/policies. |
| S06 | External I/O | File encodings/layout, standard streams, path resolution, and I/O-error policies need explicit choices in `106`-`108`. Never embed open host handles in artifacts. |
| S07 | Waveform format limits | VCD/FST mappings may not expose all VHDL states/types identically. Define and document mappings/rejection and same-time/delta observation policy in `113`-`118`; never change simulation values or invent positive delays. |
| S08 | Artifact compatibility | Binary is default; RON exposes the same logical model. Version/stage/intrinsic/policy compatibility failures are diagnostics, not hidden reanalysis or partial decoding. Owners `007`-`010`,`121`. |

## Waveform and CI Requirements

Waveform tasks are `113`-`118`. They must cover initial values, effective signal updates after resolution, hierarchical names/aliases, index directions and widths, time resolution, filtered signals, simultaneous VCD/FST output, stop/failure finalization, and checked output-path errors. Disabled dumping must not change scheduling or retain an unbounded trace.

FST must use the Rust `fst_writer` API from Cargo package `fst-writer`. The published package checked on 2026-10-27 is `0.3.1`, BSD-3-Clause, with declared Rust `1.85.0`; [its source repository](https://github.com/ekiwi/fst-writer) is the upstream reference. This metadata check is not an API or numerical-format probe. Verify and pin the selected version/API in `115`, not as an unused dependency in chunk `002`.

Choose a suitable established VCD writer in `114`. Read back VCD and FST using independent parsers/utilities; writer success or nonempty files are insufficient. Compare decoded observations with fixtures, each other where applicable, and reference traces. Do not require raw compressed-file identity or assume viewers preserve every same-time transition. Nine-state logic, weak values, enum labels, real/fixed/floating/complex values, and unsupported objects need explicit dump policies, separate from their exact internal simulation semantics.

GitHub Actions is the CI target. Chunk `004` creates pull-request/push workflows with formatting, compilation, Clippy, tests, pinned tools/actions, least-privilege permissions, and diagnostic path portability. Chunk `030` adds required differential jobs; `118` adds waveform readers/parity and useful failure artifacts. Jobs must not expose secrets to untrusted pull requests or count missing required tools as passing/skipped conformance. User-local reference tools may be optional only when the skip is reported honestly.

## Reproduce the Inventory

The executable tool is [../probes/parser/examples/inventory.rs](../probes/parser/examples/inventory.rs). It extends the existing isolated probe rather than bootstrapping the simulator or introducing standard-library runtime dependencies.

Set `RUST_HDL_ROOT`, `GHDL_ROOT`, and `NVC_ROOT` to checkouts of the revisions above. The parser checkout is downloaded by Cargo using the existing lockfile; its manifest path can be located with `cargo metadata`. The reference repositories are audit inputs only and can be cached outside the workspace.

For fresh source references, use detached pinned checkouts; sparse checkout may select only `libraries/std`, `libraries/ieee2008`, and `src/vhdl` in GHDL, and `lib/std.08`, `lib/ieee.08`, `lib/ieee`, plus optionally `lib/std.19` for the revision audit in NVC. Never substitute NVC's older `lib/ieee` numeric/logic files for its `ieee.08` interfaces; only the inherited math packages come from the common directory.

```sh
cargo fmt --manifest-path probes/parser/Cargo.toml
cargo check --manifest-path probes/parser/Cargo.toml --all-targets --locked
cargo clippy --manifest-path probes/parser/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path probes/parser/Cargo.toml --all-targets --locked
cargo run --quiet --manifest-path probes/parser/Cargo.toml --locked --example inventory -- \
  --check "$RUST_HDL_ROOT" "$GHDL_ROOT" "$NVC_ROOT" docs/native-declarations.md
```

To inspect one declaration file, supply its path as the only argument to the example. To regenerate after a deliberate source/ownership update, replace `--check` with `--write`. Report generation is token/AST-based and deterministic; `--check` does not rewrite files. It fails on changed source revisions/blobs, syntax errors, unbalanced template markers, unexpected duplicate/empty units, unassigned functions, unreconciled functional Rust-HDL/GHDL IEEE profiles, or any stale report content.

The inventory-tool tests exercise overload/default/generic/alias retention, literal case and comment removal, malformed/body rejection, 2008 template selection with preserved positions, stale-report detection, unassigned functions, formatted real text output versus excluded later file helpers, and completeness of the required unit/source list. These tests validate inventory machinery, not native package implementation. The actual pinned VHDL declaration files provide the large fixture-driven audit.

Before closing chunk `002`, run the commands above, validate that every owner points to a current plan chunk, verify matrix/test-ID coverage and diagnostic portability requirements, check document diagnostics, and record exact results in the plan handoff. Do not begin codec/backend decisions from `003` or mark future package/waveform/CI implementations complete.

## Verified Handoff

Verified on 2026-10-27: formatting, `cargo check --all-targets --locked`, strict Clippy, and all-target tests pass using the probe manifest. There are 16 passing tests: eight original parser checks and eight inventory-tool checks. The report's `--check` passes against all three pinned source checkouts.

The cross-document audit verifies 130 consecutive plan chunks, 68 ordered language/workflow rows with diagnostic/test columns, 18 required package/context units, and 1,870 unique audited profiles/unit rows, with all owner references valid. Document/editor diagnostics are clean. The test corpus is not a claim that native VHDL operations, waveform output, or GitHub Actions workflows already execute; those are explicitly assigned future work.

Chunk `002` is complete and `003` remains unstarted. No production public contract or dependency lockfile was changed by this chunk, and no standard-package-body runtime dependency was introduced.
