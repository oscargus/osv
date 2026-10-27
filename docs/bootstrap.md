# Rust Bootstrap and CI

Plan chunk `004`. The root package is one unpublished `osv` library and binary,
not an implemented VHDL analyzer or simulator. The execution/codec decisions are
in [execution-and-artifacts.md](execution-and-artifacts.md). The earlier isolated
probe keeps its own workspace, manifest, and lockfile.

## Toolchain and Dependencies

[../rust-toolchain.toml](../rust-toolchain.toml) pins Rust `1.98.0` with Rustfmt
and Clippy. [../Cargo.toml](../Cargo.toml) declares minimum supported Rust `1.98`
and edition 2024. The minimum and the development/CI pin are the same tested
release; no older compiler support is claimed. Upstream's `1.95` declaration is
not OSV's MSRV guarantee. Change the MSRV/pin deliberately and rerun all gates.

The root [../Cargo.lock](../Cargo.lock) deliberately preserves
`vhdl_syntax` at explicit Git revision
`dc8c090f7aeb9a95149eadc5e8b48f170382a8d4`, verified as the latest upstream
`master` commit on 2026-10-27. Both manifests use that `rev`; no moving branch
is selected at build time. Ordinary commands use `--locked`.

Registry dependencies use compatible version ranges rather than exact `=`
requirements. Current lockfiles resolve Serde `1.0.229`, Postcard `1.1.3` with
only `alloc`, RON `0.12.2`, Ariadne `0.6.0` without automatic color detection,
and Clap `4.6.7` with derives and generated help/usage but without color.
Refresh lockfiles deliberately with `cargo update` in each workspace and rerun
their checks. Only the requested Git revision is explicit in the manifests;
lockfiles retain reproducible registry resolutions. The rejected Bincode probe
dependency was removed because its latest release, `3.0.0`, intentionally fails
compilation and provides no codec API. The historical comparison remains in
[execution-and-artifacts.md](execution-and-artifacts.md).

No numerical, waveform, backend, or package-body dependency was added. The
library and binary forbid unsafe Rust.

## Bootstrap Contracts

[../src/lib.rs](../src/lib.rs) reserves these module ownership boundaries without
pretend implementations:

| Module | First Implementation Owner |
| --- | --- |
| `source`, `diagnostics` | `005`: shared source and diagnostic model. |
| `syntax` | `015`: local VHDL 2008 parser adapter. |
| `analysis` | `016` onward: owned semantic data and checks. |
| `artifacts` | `007`-`010`: owned schema, codecs, commands, and catalog. |
| `elaboration` | `022` onward: concrete executable designs. |
| `values` | `011`-`014`: checked scalars, bounds, and vector storage. |
| `intrinsics` | `017`: trusted declaration-based registry. |
| `lowering` | `023`: compiled continuations and operation selection. |
| `runtime` | `025` onward: scheduling, drivers, and process execution. |
| `cli` | Bootstrap now; stage commands land in their assigned later chunks. |

[../src/cli.rs](../src/cli.rs) exposes
`run(arguments, output, errors) -> io::Result<u8>`, with injected writers and
typed Clap derives. No arguments or `-h`/`--help` prints generated help and
returns zero; `--version` identifies the bootstrap. Nested `artifact inspect`
and `artifact convert` commands are registered, with help at every command level
and Clap's normal help/version display precedence. Parsed stage/artifact commands
still return status `2` with `OSV-CLI-UNIMPLEMENTED`; unknown commands/options
return `2` with `OSV-CLI-USAGE` and Clap's usage detail. Opaque trailing stage
arguments are accepted only to report that the command is unimplemented, not to
claim its future options work. No stage reads sources, resolves a library,
writes an artifact, or produces simulation output. The binary also maps output
failures to a nonzero `OSV-CLI-IO` diagnostic.

[../src/diagnostics.rs](../src/diagnostics.rs) provides only
`display_path(&Path) -> String`: lossy host-path display with separators
normalized to `/`, preserving drive/UNC prefixes and relative components. This
is **not** canonical filesystem identity, path resolution, a source manager, a
byte-span model, or a full renderer. Do not use display labels as artifact IDs or
file-open paths. Chunk `005` owns the structured diagnostic interfaces.

The eight integration smoke tests in [../tests/cli.rs](../tests/cli.rs) execute the
real binary and check generated/nested help, unavailable stages, usage failures,
version, and Clap's display precedence.
They also parse malformed ASCII VHDL in explicit 2008 mode and render the actual
parser span with Ariadne, checking byte-for-byte identical, color-free output
for slash/backslash labels and retention of drive/UNC prefixes. This is a
synthetic integration smoke test, not the production parser diagnostic adapter
or the golden-fixture harness from later chunks.

## Local Checks

From the repository root:

```sh
cargo fmt --all
cargo build --workspace --all-targets --locked
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
cargo run --locked -- --help
cargo fmt --manifest-path probes/parser/Cargo.toml --all
cargo check --manifest-path probes/parser/Cargo.toml --all-targets --locked
cargo clippy --manifest-path probes/parser/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path probes/parser/Cargo.toml --all-targets --locked
```

Root `--workspace` commands intentionally exclude the isolated probe. Its
separate gates are mandatory; see [coverage.md](coverage.md) for the additional
declaration report `--check` command and exact reference-source roots.

### Local Results

Verified on 2026-10-27 with Rust `1.98.0`: root formatting, library/binary build,
check, strict Clippy and all eight integration tests pass. Probe formatting,
check, strict Clippy and all 23 tests pass. The pinned-source declaration report
`--check` matches all 1,870 profiles. Checksum-verified Actionlint accepts the
workflow; editor diagnostics and patch hygiene checks are clean.

Before the Clap/dependency follow-up, a source export without Git metadata or
previous build outputs was tested with an empty `CARGO_HOME` and target
directory. `cargo test --workspace --all-targets --locked` fetched dependencies,
built the library/binary, and passed the original six integration tests. That
historical result is not fresh-cache certification of the updated dependencies,
nor hosted or non-Linux certification.

## GitHub Actions

[../.github/workflows/ci.yml](../.github/workflows/ci.yml) runs on `push` and
`pull_request`, with `contents: read`, no additional secrets, no persisted
checkout credentials, bounded job times, and cancellation of superseded runs.
Official checkout `v5.0.0` and cache `v5.0.0` actions are pinned to verified commit
SHAs, not moving tags. Hosted runners supply the required Node-24-capable runner;
self-hosted runner support has not been certified. Rust installation names
`1.98.0` explicitly. Cache keys include OS, architecture, toolchain, manifests,
and lockfiles; production/probe target caches are separate.

| Job | Required Entry Points |
| --- | --- |
| Formatting | Nonmutating `cargo fmt --all -- --check` for both workspaces. |
| Rust | Build/check/strict Clippy/all-target tests/help on `ubuntu-24.04`, `ubuntu-24.04-arm`, `windows-2022`, and `macos-15`; every runner executes diagnostic portability tests. |
| Probes and declaration inventory | Check/Clippy/all 23 probe tests, then regenerate-and-compare the 1,870-profile report against separately checked-out, exact Rust-HDL/GHDL/NVC revisions. These are declaration sources, not reference executables or standard-body runtime fallbacks. |
| Workflow syntax | Actionlint `1.7.7`, downloaded from its exact release and checked against SHA-256 `023070a287cd8cccd71515fedc843f1985bf96c436b7effaecce67290e7e0757`. Optional ShellCheck/Pyflakes integrations are explicitly disabled, not claimed as checks. |

No job tolerates a missing required tool/source, uses `continue-on-error`, or
turns a reference-tool skip into a success. The inventory validates source
provenance and coverage records, **not** implemented native package semantics.
Required GHDL execution/differential jobs remain owned by `030`; waveform
writers/readers/parity jobs by `114`-`118`; native runtime closure by `104`/`122`.
No placeholder jobs claim these features pass.

Maintainers must configure these job names as required status checks in GitHub
branch protection; workflow files cannot set that repository administration
policy. At implementation time this checkout has no Git remote, so hosted job
execution, non-Linux results, and branch-protection settings cannot be verified
locally. Local syntax/command checks are not a hosted CI pass. Keep chunk `004`
pending until its required hosted smoke/matrix results have been observed.
