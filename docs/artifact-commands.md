# Artifact Commands

Plan chunk `009`. Inspection and conversion are available; VHDL analysis,
elaboration, and simulation remain explicit bootstrap errors. These commands
operate on the owned models from [artifact-model.md](artifact-model.md) through
the bounded readers described in [artifact-codecs.md](artifact-codecs.md).
They never parse sources or elaborate a design to repair an input.

```sh
cargo run --locked -- artifact inspect tests/fixtures/artifacts/analysis.ron
cargo run --locked -- artifact convert tests/fixtures/artifacts/analysis.ron --to binary --output synthetic.osva
cargo run --locked -- artifact convert synthetic.osva --to ron --output synthetic-copy.ron
```

The checked-in input is explicitly synthetic, not an analyzed VHDL design.
Output targets must not already exist. Inspection prints canonical, readable,
named RON to standard output and does not write to its input. Conversion
requires typed `--to binary|ron` and `--output PATH` options and writes nothing
to standard output on success. Both commands detect input format by contents,
regardless of path extensions, and preserve logical fingerprints. Binary is
the default encoding at the shared format API; conversion explicitly names
its target rather than guessing from an extension.

## Safe Filesystem Boundary

[../src/artifacts/files.rs](../src/artifacts/files.rs) exposes shared helpers:

- `load(path, expected_stage, limits)` opens an artifact and calls the existing
  bounded reader. It never follows saved source display names or repairs
  stale dependencies by reanalysis.
- `publish_new(path, bytes)` creates a temporary file in the output directory,
  writes and synchronizes its complete contents, then publishes with
  `tempfile`'s no-clobber persistence. It refuses existing files, directories,
  and symlinks, including input aliases. Parent directories must already
  exist. Ordinary failures clean temporary files without publishing partial
  output. No overwrite or implicit force option is provided.

Conversion finishes decoding, validating, and encoding before publication.
There is no check-then-truncate race: existing-target rejection happens at
publication itself. The helper is shared with future stages; atomic catalog
replacement is a separate `010` responsibility. Synchronizing file contents
does not promise directory-entry durability across a machine power failure.
An uncatchable process kill may leave a temporary file, but cannot publish
partially encoded output through this path.

The established `tempfile` crate is resolved to `3.27.0` with compatible major
requirement `3`; root lockfiles retain all parser/codec pins. No VHDL package
body is involved in loading, inspection, or conversion.

## Diagnostics and Verification

Argument/option errors remain Clap usage errors with status `2`. File, codec,
compatibility, validation, and publication failures use shared structured
artifact diagnostics with status `1` and normalized input/output path notes.
Success returns `0`. The injected output/error writer contract of `cli::run`
is unchanged, including propagation of writer I/O errors.

[../tests/artifact_commands.rs](../tests/artifact_commands.rs) uses isolated
temporary directories and fresh CLI processes to verify both conversions,
misleading extensions, fingerprint and deterministic-binary preservation,
nonmutating readable inspection, input/existing-output collisions, malformed
and incompatible input rejection, required/invalid options, and cleanup after
publication failure. Existing CLI smoke tests retain explicit unavailable
analysis/elaboration/simulation errors, help/version behavior, and diagnostic
path portability.

```sh
cargo test --test artifact_commands --test cli --locked
cargo fmt --all
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
```
