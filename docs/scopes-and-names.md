# Initial Scopes and Declarations

Plan chunk `016`, with persistence child `016a` and source-registration child
`016b`. Owned scope identity is documented in
[artifact-model.md](artifact-model.md). Name APIs live in
[../src/analysis/names.rs](../src/analysis/names.rs) and
[../src/analysis/units.rs](../src/analysis/units.rs).

## Declaration Registration

`UnitRegistry::new(library)` creates the selected library scope and initial
`std` library visibility. The `work` alias always means that selected current
library, not a hardcoded physical library named `work`. Basic names normalize
case; extended names retain case, remain distinct from basic names, and decode
escaped backslashes from the parser-validated token. Identifier legality comes
from the pinned lexer/parser, not a second hand-written lexer.

`register(&ParsedSource)` consumes local syntax views in source order. It
registers entities, architectures, packages, package bodies, configurations,
and context unit headers, assigning stable local `UnitId`/`ScopeId` values.
Primary names share one library namespace; architectures and package bodies
remain independently identified secondaries. A secondary requires an earlier
matching entity/package, including across files. Missing and wrong-kind parents
have distinct diagnostics and related source locations when available.

Registration is transactional per call: duplicates or missing prerequisites
leave the prior registry unchanged. IDs allocate after the existing high-water
mark with explicit exhaustion errors rather than wrapping. Unit, scope, and
name source references retain the caller's shared source IDs; callers must
maintain that source table across files.

The name pass also collects supported type/subtype, constant/signal/variable,
and ordinary named subprogram declarations. Process, block, and subprogram-body
lexical scopes retain their owning unit and parent. Ordinary homographs within
one scope fail with both declaration locations; inner declarations shadow
parents. Subprogram candidates remain a set for later signature/body/overload
checking, never a silently selected first spelling.

`DeclarationName` is explicitly **name-pass data**: ID, owner, scope, name,
declaration kind, and optional source location. It has no guessed type, value,
initializer, executable body, or runtime semantics and is not an analyzed
artifact payload. Later passes must resolve/check types and statements before
publishing analyzed units. Configurations/context headers are registered as
names, not implemented binding or transitive visibility behavior. Unsupported
semantic bodies do not become operations/no-ops merely because their names
were collected. CLI `analyze` remains unavailable until `021`.

## Lookup and Persistence

`lookup_unit(library, primary, kind)` resolves complete library/unit identity,
including `work` and normalized architecture names. `library_scope` exposes
available library identity, not a claim that its package contents exist.
The standard prelude and native declaration identities remain `017`.

`UnitRegistry::lookup` and `names::ScopeLookup::lookup` walk nearest lexical
scope first. The latter requires a validated artifact and returns its actual
`SymbolRecord` references. Missing scopes are errors; absent names yield no
candidates. Both return all candidates in the first matching scope, preserving
overload resolution for its real owning pass. They do not search arbitrary
unqualified strings across libraries.

`UnitRegistry::from_artifact` restores validated unit/scope/symbol identities
without renumbering, opening sources, or inventing missing source locations.
Saved symbol IDs therefore remain the lookup result after binary/RON reload.
The persisted scope fields remain those introduced in schema `2` by `016a`;
schema `3` adds native binding data in `017b`. Transient syntax views and
registration caches are never serialized into the artifact.

The initial context subset recognizes available `std`/current-library clauses.
Unavailable libraries fail explicitly. Full `use`-clause visibility currently
returns a source-linked unsupported diagnostic with coverage `L006`, rather
than pretending to import names. Later `034`/`039` own full imports, homographs,
contexts, and overload rules. These limits are deliberate; no standard package
body is loaded and no name-only user function is substituted by a native body.

## Checks

[../tests/names.rs](../tests/names.rs) uses VHDL fixtures for primary/secondary
registration, declarations and lexical shadowing, duplicates/rollback, basic
and extended names, separate-file prerequisites/source IDs, primary namespace
collisions, wrong parents, library aliases, and explicit visibility limits.
Artifact-backed lookup tests independently verify symbol identity after both
codecs and preserve unavailable locations rather than fabricating byte zero.

```sh
cargo test --test names --test artifact_scopes --locked
cargo fmt --all
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
```
