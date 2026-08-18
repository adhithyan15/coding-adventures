# `coding_adventures_vault_pm_application_storage_core`

This crate supplies the missing durable-host bridge between the host-neutral
VLT-PM05 application and an injected `storage-core` backend. It implements:

- `BootstrapStore`, with immutable signed generations, one atomic latest
  pointer per random bootstrap locator, and an explicit supersession that
  removes a retired generation; and
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

## Why a retired generation is deleted

`supersede_generation` exists because "immutable generations" and "the old
passphrase must stop working" are in tension, and only one of them can win.

Every generation is one wrapping of the vault root key under one
passphrase-derived key. A passphrase rotation installs a new wrapping of the
*same, unchanged* root key and moves the latest pointer — so if the previous
generation stayed on disk, anyone who later obtained a copy of this directory
plus the retired passphrase could unwrap that root key from it and derive every
subkey. The rotation would have accomplished nothing against the adversary the
person most likely had in mind. Superseded has to mean gone.

Nothing is lost by removing it. Each bootstrap still names its predecessor by
hash, so the chain stays linked and a rollback stays detectable, and nothing in
this product ever reads a non-latest generation.

Two behaviours are load-bearing. The generation the latest pointer names is
refused outright with `Conflict` — that record is the only way into the vault,
so a guard is worth more than a convention that every caller passes the right
identifier. And an already-absent record is success, because a rotation's
recovery replays this call after a crash and must be able to reach the end. The
removal is read back afterwards, so a delete that silently did not happen
reports `Corruption` rather than success.

## Verification

Twelve tests cover generation-zero installation, rotation, idempotent retry,
stale and competing predecessors, malformed generation/pointer data,
supersession of a retired generation and refusal of the live one, exact
local-state CAS, concurrent writers, closed errors, and filesystem
reconstruction.

```bash
bash BUILD
cargo clippy -p coding_adventures_vault_pm_application_storage_core --all-targets -- -D warnings
```
