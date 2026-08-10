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

Diagnostics and `Debug` output never include resolved paths.

Eleven tests cover path resolution, idempotent layout creation, native modes and
object types, broad-root and unsafe-file refusal, stable redaction, lock
contention, exact configuration round trips, stale-write rejection, and input
bounds. Windows executes the same public contract in CI and cross-target Clippy
validates its native API surface locally. Tarpaulin's LLVM engine measures 304
of 319 Unix production lines covered (95.30%).

## Verification

```bash
bash BUILD
cargo clippy -p coding_adventures_vault_pm_local_host --all-targets -- -D warnings
```
