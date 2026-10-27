# Parser and Standard Source Verification

Plan chunk `001`, verified on 2026-10-27. The isolated probe verifies a parser integration path; it is not the simulator bootstrap from chunk `004`.

## Dependency and Toolchain

Use an explicit commit resolved from upstream `master`, as requested, rather
than the published crates.io release. The latest commit was rechecked on
2026-10-27 and is unchanged from the original probe:

```toml
vhdl_syntax = { git = "https://github.com/VHDL-LS/rust_hdl.git", rev = "dc8c090f7aeb9a95149eadc5e8b48f170382a8d4" }
```

The probe manifest is [../probes/parser/Cargo.toml](../probes/parser/Cargo.toml). Its [../probes/parser/Cargo.lock](../probes/parser/Cargo.lock) resolves the branch to:

```text
dc8c090f7aeb9a95149eadc5e8b48f170382a8d4
```

- Repository: <https://github.com/VHDL-LS/rust_hdl.git>.
- Crate metadata version at this revision: `0.2.0`. This is a Git dependency containing unpublished changes, not the crates.io `0.2.0` release.
- Declared minimum Rust version: `1.95`, inherited from the upstream workspace. The published release's minimum of `1.87` does not apply to this checkout.
- Verified local toolchain: `rustc 1.98.0 (88d9e12ae 2026-08-18)`, host `x86_64-unknown-linux-gnu`. The declared minimum itself has not been tested.
- Parser license: MPL-2.0, confirmed in the repository license notice and parser source headers.
- Optional parser Serde support is not enabled. OSV's artifact model must remain independent of parser implementation types.

Both root and probe manifests now select that `rev` explicitly. Retain the
lockfiles and use `--locked` for ordinary validation. Updating to a newer
`master` commit requires changing both manifests and deliberately refreshing
both lockfiles, then rerunning the parser/decision probes and declaration audit;
it must not happen silently during normal builds. Registry dependencies use
ordinary compatible version ranges instead of exact version pins.

## Verified Parser Boundary

The byte-oriented entry point is:

```rust
use vhdl_syntax::{parser, standard::VHDLStandard};

let (design, diagnostics) =
    parser::parse_with_standard(VHDLStandard::VHDL2008, source_bytes);
```

- Result types are `syntax::DesignFileSyntax` and `Vec<parser::error::SyntaxErr>`.
- Select `VHDL2008` explicitly. The probe does not rely on the current default.
- `syntax::AstNode` provides access to the underlying lossless tree through `raw()`; typed syntax views and generated node kinds are available.
- The observed first child of an entity design unit is `EntityDeclaration`; an architecture is `ArchitectureBody`. A primary package has the wrapper `PrimaryUnitPackageDeclaration`, not a directly exposed `PackageDeclaration` at that level.
- `raw().write_to(...)` reproduces the input bytes, including trivia and malformed input in the tested cases.
- `range()` covers a node including leading trivia; `text_range()` excludes leading trivia. Ranges are byte-based and half-open in the tested implementation. A one-byte illegal input produces `17..18`; a missing construct can have a zero-width insertion span such as `16..16`.
- `SyntaxErr::span()` and `SyntaxErr::err()` expose structured locations and kinds. `parser::error::display_errors(...)` provides a simple textual renderer; OSV should map the structured errors to its own diagnostics instead of parsing that text.
- `text::source_loc` and the encoding utilities provide source-location conversion facilities. Their API was inspected, but the probe tests raw byte ranges, not the eventual `ariadne` integration.

The upstream error module's introductory prose describes inclusive endpoints, but its range type, range construction, and the checked one-byte diagnostic use half-open Rust ranges. Preserve zero-width spans and test conversion at the OSV adapter boundary rather than copying the prose literally.

## Encoding and Semantic Limits

VHDL source is Latin-1 except for the treatment of comments described by the parser. Prefer raw bytes over `read_to_string` or an implicit UTF-8 conversion. The probe verifies preservation of a Latin-1 byte and CRLF line endings, with the entity starting at byte offset `9` after that comment.

The parser offers syntax-tree validation, including `parse_valid_with_standard`, but this is not semantic analysis. The probe successfully parses a design with an undeclared type and a boolean initialized with the integer literal `42`; OSV must still perform name resolution, overload selection, staticness, type/width checks, and elaboration.

The sibling `vhdl_lang` crate contains semantic-analysis/project facilities and publicly exports types such as `Project`, `Type`, and `AnyEnt`. It also exports a separate `VHDLParser`/AST interface. This probe does not depend on it, and compatibility with OSV's chosen syntax/IR boundary has not been established. Reuse decisions must be explicit rather than assuming it is a drop-in semantic layer over `vhdl_syntax`.

The parser's own documentation states that non-2008 selection is largely a placeholder and that PSL statements are not supported. The probe verifies a 2008 `process(all)` and rejection of a 2019 mode-view declaration under 2008 selection. Those cases are not a complete language-revision or PSL conformance claim. Carry those boundaries into the chunk `002` feature matrix and chunk `015` adapter tests.

## Versioned Standard Declaration Sources

All paths below are within the same upstream revision recorded above. No standard declaration or package-body source has been copied into OSV by this chunk.

| Library | Required Declarations | Upstream Location |
| --- | --- | --- |
| `std` | `standard`, `textio`, `env` | `vhdl_libraries/std/standard.vhd`, `textio.vhd`, `env.vhd` |
| `ieee` | `std_logic_1164`, `numeric_std`, `math_real` | Corresponding declaration files in `vhdl_libraries/ieee2008/` |
| `ieee` | `fixed_pkg`, `float_pkg` | Instantiated package declarations in `vhdl_libraries/ieee2008/` |
| `ieee` dependencies | `fixed_float_types`, `fixed_generic_pkg`, `float_generic_pkg` | Corresponding declaration files in `vhdl_libraries/ieee2008/`; audit exact dependencies in chunk `002` |

Pinned source directories:

- [Standard declarations](https://github.com/VHDL-LS/rust_hdl/tree/dc8c090f7aeb9a95149eadc5e8b48f170382a8d4/vhdl_libraries/std).
- [IEEE declarations](https://github.com/VHDL-LS/rust_hdl/tree/dc8c090f7aeb9a95149eadc5e8b48f170382a8d4/vhdl_libraries/ieee2008).
- [Repository license notice](https://github.com/VHDL-LS/rust_hdl/blob/dc8c090f7aeb9a95149eadc5e8b48f170382a8d4/LICENSE.txt).

The required IEEE files and listed fixed/floating dependencies carry Apache-2.0 headers and IEEE attribution. Preserve those notices separately from the parser's MPL-2.0 licensing. The `std` files have no individual inline license notice; the enclosing repository has an MPL-2.0 notice. Retain the repository provenance and review attribution/licensing before any vendoring or redistribution. This chunk references the files without redistributing them.

The standard package name is `ieee.std_logic_1164`, as confirmed by the upstream filename and package declaration; do not introduce a `std_1164` alias silently.

These are useful, versioned declaration references, not proof of a complete, exclusively 2008 interface. A language-server declaration file or a directory named `ieee2008` can be abbreviated or incorporate later changes. Chunk `002` must reconcile every signature, default, alias, implicit predefined operation, and generic policy with a VHDL 2008 reference. Do not infer the revision solely from a header or its copyright year. Compiler-provided operations absent from these files still belong in the inventory.

`std.standard`, `std.textio`, and `std.env` are all required, not optional packages. Native implementations must cover the selected 2008 interfaces without requiring their VHDL package bodies. User-written package bodies remain ordinary VHDL.

## Reproduce the Checks

Run from the workspace root:

```sh
cargo test --manifest-path probes/parser/Cargo.toml --locked -- --nocapture
cargo fmt --manifest-path probes/parser/Cargo.toml -- --check
cargo clippy --manifest-path probes/parser/Cargo.toml --all-targets --locked -- -D warnings
```

All commands passed. The test suite contains eight checks:

1. Entity and architecture parsing, expected unit kinds, and byte-for-byte reproduction.
2. Package parsing and the primary-package wrapper kind.
3. Explicit 2008 parsing of `process(all)`.
4. Latin-1 byte preservation, CRLF preservation, and a node byte offset.
5. Demonstration that semantic name/type errors are not parser errors.
6. Malformed-input recovery, structured error kind, zero-width span, and message.
7. A nonempty half-open diagnostic byte span.
8. Rejection of a 2019 mode-view declaration under 2008 selection.

Representative malformed input is `entity broken is\n`. Its one diagnostic is:

```text
16..16 expected EntityDeclarationEpilogue
```

No reference simulator is required for this parser-only chunk. GHDL was unavailable when checked; obtaining/pinning it remains part of the later differential-testing work. No production simulator behavior or full standard-library support is claimed here.

## Handoff

Chunk `001` is complete: the requested Git parser builds, the minimal integration and diagnostics are tested, and versioned declaration sources and their license notices are identified. There is no blocker to beginning the inventories.

Next is chunk `002`, including every VHDL 2008 `std` package. Track parser revision/PSL limitations, declaration completeness/provenance, implicit predefined operations, and exact generic-package dependencies explicitly. Do not start the production Cargo package, replace the parser, or copy standard package bodies as part of this handoff.
