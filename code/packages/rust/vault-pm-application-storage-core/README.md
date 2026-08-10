# `coding_adventures_vault_pm_application_storage_core`

This crate supplies the missing durable-host bridge between the host-neutral
VLT-PM05 application and an injected `storage-core` backend. It implements:

- `BootstrapStore`, with immutable signed generations and one atomic latest
  pointer per random bootstrap locator; and
- `LocalStateStore`, with exact byte compare-and-exchange for owner-private
  initialization and publication journals.

Names are domain-separated hashes or lowercase hex of random/content-derived
identifiers. Record metadata is empty, diagnostics are closed, and no vault ID,
title, username, record type, provider path, or state bytes enter storage names
or errors.

The adapter owns no filesystem path. A local CLI composes it over a separately
permission-checked `FsStorageBackend`; a future SQLite or native-preferences
host can reuse the same VLT-PM05 traits without changing the application.

## Crash and race behavior

Bootstrap generations are written immutably before the latest pointer moves.
An interruption can therefore leave only an unreachable generation, never a
pointer to missing bytes. Pointer races re-read the winner, making exact retries
idempotent while rejecting a different successor.

Local owner state uses the backend's atomic conditional create or revision CAS.
The adapter compares complete bytes before every replacement and reads back the
committed value. One adapter-level write lock keeps that read/condition/write
sequence indivisible even when a backend restarts its revision-token sequence.
It never overwrites a different winner. Phase 1A still requires one adapter
instance and the CLI host's documented single-writer process exclusion because
`storage-core` does not promote conditions across backend instances.

## Verification

Eleven tests cover generation-zero installation, rotation, idempotent retry,
stale and competing predecessors, malformed generation/pointer data, exact
local-state CAS, concurrent writers, closed errors, and filesystem
reconstruction. Tarpaulin's LLVM engine measures 225 of 236 production lines
covered (95.34%).

```bash
bash BUILD
cargo clippy -p coding_adventures_vault_pm_application_storage_core --all-targets -- -D warnings
```
