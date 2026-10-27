# Library Artifact Store

Plan chunk `010`. The store uses synthetic analyzed records until real analysis
lands. It does not parse sources, load package bodies, elaborate, or simulate.
Implementation is split across
[../src/artifacts/store.rs](../src/artifacts/store.rs),
[../src/artifacts/store/catalog.rs](../src/artifacts/store/catalog.rs), and
[../src/artifacts/store/graph.rs](../src/artifacts/store/graph.rs).

## Layout and Catalogs

`LibraryStore::open(root, library, options)` opens a per-library view of a shared
store root. Missing catalogs yield an empty view without creating entries.
Immutable objects live in `objects/<logical-fingerprint>.osva`; each is one
individually encoded analysis artifact, not a monolithic library payload.
Both binary and RON are accepted. The fixed object extension is not a codec
signal. Reusing an existing logical fingerprint validates the saved object and
retains its original encoding; converting formats cannot change its identity.

Each library has a versioned binary catalog under
`libraries/<library-identity-fingerprint>.osvc`. Library path components derive
from typed canonical identity, not user-controlled path fragments. The catalog
has `OSVCAT\0\0` magic and a Serde payload containing catalog version `1`,
artifact schema and compiler compatibility requirements, library identity,
available `(unit identity, fingerprint, language)` entries, and separate active
bindings. It reuses the bounded binary decoding layer and rejects incompatible,
duplicate, dangling, truncated, or trailing data. It is store metadata, not an
analysis/elaboration artifact or an alternative logical artifact codec.

Primary units, each named architecture, and package bodies retain distinct
identities and versions. Secondary analysis payloads retain exact primary
declaration dependencies. Catalog replacement changes active bindings without
deleting older objects or versions. Body changes therefore do not replace the
package declaration identity. Current analysis options are the selected library
and sole `Vhdl2008` revision already retained in identity/envelope/catalog;
unsupported optimization or revision switches are not invented. Future semantic
options must enter the versioned artifact identity, not only a timestamp or
unhashed catalog setting.

## Publication

`publish(artifact, format)` validates the analysis stage, library, native
requirements, exact dependency closure, and absence of conflicting versions
before publishing. A same-directory temporary file and no-clobber publication
create the complete immutable object first. Then the catalog is serialized,
written to its own same-directory temporary file, synchronized, and atomically
replaced. The in-memory snapshot changes only after success.

An OS file lock serializes writers across the root. Under that lock, publication
rechecks the catalog snapshot. A stale writer gets `ConcurrentUpdate` and must
reopen; it cannot overwrite another writer's bindings. Locks release when the
file handle drops, including on errors. Partial temporary files are not visible
catalogs. An interrupted writer may leave an unreferenced complete object;
readers ignore it because it is not cataloged. Cleanup/garbage collection of
orphaned immutable objects is not implemented. File synchronization does not
promise directory-entry persistence after a power failure.

## Resolution and Invalidation

- `catalog()` exposes immutable metadata; `lookup` uses complete normalized
  unit identity, including architecture name or package-body kind.
- `resolve_current(identity)` starts from the active binding and requires
  every saved dependency fingerprint to match its current binding. A changed
  declaration or explicitly required body is a stale dependency, never hidden
  reanalysis or automatic rebinding.
- `resolve_exact(identity, fingerprint)` resolves a reproducible historical
  graph from retained catalog versions, regardless of newer active bindings.
  Missing, incompatible, corrupt, cyclic, or conflicting versions still fail.
- `artifact_path(fingerprint)` and `catalog_path()` expose deterministic paths
  for callers/tooling, not an alternative to validated load/resolve operations.

Both graph paths load exact owned artifacts through the same bounded codecs,
compare logical identity/fingerprint against the catalog, and enforce native
declaration-set/ABI compatibility. Cross-library dependencies resolve through
the corresponding catalogs under the same root. Graph traversal is iterative,
retains one version of each logical unit, and returns deterministic unit-identity
ordering. It never uses source-file existence or timestamps for validity.

A specification-only consumer survives a body-only update. A consumer that
explicitly records a body dependency becomes stale when that body changes.
Historical exact graphs remain available provided their native implementation
requirements are still available. Schema/compiler changes reject stale catalogs
explicitly; there is no migration or best-effort reuse.

`StoreOptions` supplies available native declaration fingerprints/ABIs, the
shared per-object decode limits, and graph limits (default 1,024 artifacts and
256 MiB cumulative encoded object bytes). This is separate from the per-object
decoded-allocation budget. Native policy cannot contain ambiguous duplicate
package identities. `StoreError::diagnostic()` produces shared artifact-stage
errors; absent references and stale versions never count as a cache hit.

## Verification

[../tests/artifact_store.rs](../tests/artifact_store.rs) covers exact
architecture identity, fresh/reused/reopened entries, binary/RON objects,
declaration and body-only updates, historical graphs, cross-library references,
missing/corrupt objects, stale writers, interrupted temporary/orphan writes,
native declaration changes, schema rejection, conflicting graph versions,
resource limits, and structured diagnostics.

```sh
cargo test --test artifact_store --locked
cargo fmt --all
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
cargo fmt --manifest-path probes/parser/Cargo.toml --all
cargo check --manifest-path probes/parser/Cargo.toml --all-targets --locked
cargo clippy --manifest-path probes/parser/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path probes/parser/Cargo.toml --all-targets --locked
```
