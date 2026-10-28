# OSV

[![CI](https://img.shields.io/github/actions/workflow/status/oscargus/osv/ci.yml?branch=master&label=CI)](https://github.com/oscargus/osv/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/Rust-1.98.0-black?logo=rust)](rust-toolchain.toml)
[![VHDL](https://img.shields.io/badge/VHDL-2008-blue)](project.md)
[![License](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-green)](#license)

OSV is a VHDL 2008 simulator being built in Rust. The goal is an event-driven
compiled simulator with separate analysis, elaboration, and simulation stages,
specialized native operations, and optional VCD and FST waveform output.

## Status

OSV is in early development and is not yet a usable end-to-end simulator.
Foundations include VHDL 2008 parsing through
[vhdl_syntax](https://github.com/VHDL-LS/rust_hdl), source diagnostics, typed
values, limited source analysis, and versioned artifacts in binary and readable
[RON](https://github.com/ron-rs/ron) formats.

The CLI can inspect and convert artifacts. Its `analyze`, `elaborate`, and
`simulate` commands are placeholders and currently return an explicit
not-implemented error. Native standard-library coverage, full simulation, and
waveform output remain under development; see the [roadmap](PLAN.md) and
[coverage tracking](docs/coverage.md).

The language target is VHDL 2008 only. CI checks Linux (x86-64 and arm64), Windows,
and macOS.

## Getting Started

Install [Rust through rustup](https://rustup.rs/), Git, and your platform's native
build tools. The repository selects Rust 1.98.0, rustfmt, and Clippy through
[rust-toolchain.toml](rust-toolchain.toml).

```sh
git clone https://github.com/oscargus/osv.git
cd osv
cargo build --locked
cargo run --locked -- --help
```

Cargo fetches the parser from a pinned Git revision of Rust-HDL. VHDL source
files are interpreted as ISO-8859-1 (Latin-1).

## Artifact Tools

Try the artifact commands with the included test fixture:

```sh
cargo run --locked -- artifact inspect tests/fixtures/artifacts/analysis.ron
cargo run --locked -- artifact convert tests/fixtures/artifacts/analysis.ron --to binary --output example.osva
cargo run --locked -- artifact inspect example.osva
```

Inspection prints readable RON. Conversion accepts binary or RON input and
supports `--to binary` and `--to ron` without reanalyzing sources. The output path
must not already exist. See [artifact commands](docs/artifact-commands.md) for
details.

## Development

```sh
cargo fmt --all
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
```

The parser probe is a separate Cargo project, excluded from the root workspace.
See the [parser probe guide](docs/parser-source-probe.md) for its checks and the
pinned standard-library declaration audit.

## Documentation

- [Project goals and requirements](project.md)
- [Implementation roadmap](PLAN.md)
- [Execution and artifact design](docs/execution-and-artifacts.md)
- [Currently supported source analysis](docs/analyzed-unit-data.md)
- [Fixture and test conventions](docs/fixtures.md)

## License

Licensed under either the [MIT License](LICENSE-MIT) or the
[Apache License, Version 2.0](LICENSE-APACHE), at your option.
