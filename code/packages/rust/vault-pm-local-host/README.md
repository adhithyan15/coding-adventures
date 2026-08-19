# `coding_adventures_vault_pm_local_host`

This crate is the operating-system trust boundary for the local password-manager
CLI. It resolves platform application directories, prepares owner-private roots
without following links, and provides non-blocking cross-process writer
exclusion before any `storage-fs` backend is opened. The resulting writer guard
also owns exact, bounded, atomic persistence for the storage-neutral VLT-PM07
configuration bytes.

It deliberately does not know the vault format, keys, records, providers, or
application use cases. The composition root receives separate paths for:

- non-secret configuration;
- owner-private application state;
- encrypted immutable objects; and
- disposable cache data.

`LocalVaultPaths::resolve` uses the platform directory resolver rather than a
literal `$HOME`. `LocalVaultPaths::prepare` creates or validates every root.
Existing roots with foreign ownership, broad permissions, links, reparse
points, or unexpected object types fail closed. It never silently repairs an
existing root; a future explicit repair ceremony remains a separate CLI action.

`PreparedLocalVault::try_acquire_writer` opens a persistent owner-only lock file
and takes a non-blocking OS lock. The guard owns the lock until drop. Every
Phase 1A command can acquire this guard before constructing independent
`storage-fs` backend instances, closing the cross-process race that backend-local
compare-and-exchange cannot close.

The writer guard loads, initially creates, and exact-value replaces
`vault-pm.toml`. It accepts only non-empty values through 64 KiB, never follows
links or reparse points, validates owner-only native security, stages writes in
the same private directory, synchronizes them, and publishes atomically.
Initial creation never replaces an existing object; stale replacement returns a
closed conflict. The host deliberately treats the bytes as opaque—the
`vault-pm-config` crate remains the sole schema and canonical-rendering owner.

`LocalVaultPaths::runtime_root`/`agent_socket_path` (VLT-PM48) resolve the
short, deterministic, owner-private directory the local agent's Unix domain
socket is bound at — beside the system temporary directory, named from a
truncated hash of the data root, and deliberately *not* nested under
`data_root`: `sockaddr_un.sun_path` is bounded to roughly 100 bytes on Linux
and macOS, far below the 4096-byte ceiling every other path in this crate
accepts, and a verbose platform data directory can already consume most of
that budget on its own. `PreparedLocalVault::ensure_runtime_root` verifies or
creates it lazily — unlike every root `prepare()` already handles, no
ordinary command reaches it — using a leaf-only check
(`ensure_private_runtime_directory`) rather than the general recursive
`ensure_private_directory`: the system temporary directory's own ancestry
(`/tmp`, `/var`) is trusted as the platform gives it, since on macOS both are
themselves platform-placed symlinks that the general walk would otherwise
refuse before ever reaching this crate's own directory.

Diagnostics and `Debug` output never include resolved paths.

Thirteen tests cover path resolution, idempotent layout creation, native modes
and object types, broad-root and unsafe-file refusal, stable redaction, lock
contention, exact configuration round trips, stale-write rejection, input
bounds, and (VLT-PM48) the runtime-socket path's determinism and owner-only
creation. Windows executes the same public contract in CI and cross-target
Clippy validates its native API surface locally. Tarpaulin's LLVM engine
measures 339 of 359 Unix production lines covered (94.43%).

## Verification

```bash
bash BUILD
cargo clippy -p coding_adventures_vault_pm_local_host --all-targets -- -D warnings
```
