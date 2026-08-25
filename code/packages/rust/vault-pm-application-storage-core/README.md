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

A third is less obvious and matters more: **the wrap is destroyed by a write,
not only by the unlink.** Every other durable step of a rotation is a write, and
a lost write is merely lost work the journal replays. This one is a removal, and
a lost removal is the opposite — it resurrects key material into a vault whose
owner state has already moved on and will never revisit it. `remove_file`
returning success proves the entry is gone from the page cache, not that its
removal is committed; on a journalling filesystem it can still be uncommitted
while a later `fsync`ed write elsewhere lands ahead of it. So the retired
record's body is first replaced with nothing through the same
write-`fsync`-`rename` path every other step uses, and only then unlinked. After
that write returns, the wrap is gone whether or not the unlink survives.

## Reclaiming a generation zero orphaned before configuration

`init` and `vault create` both install a `PreparedInit` journal under a
freshly drawn random locator *before* the caller writes the configuration
record that makes that locator discoverable again — required, so a crash
after that configuration write always leaves an exact resumable journal
behind it. The cost is the mirror case: a crash strictly *between* the two
writes leaves the journal durable under a locator nothing durable anywhere
will ever name again. It is not lost data — nothing the user created ever
existed there — but it is a permanent storage leak absent a sweep
(`VLT-PM41-cli-crash-fault-matrix.md` §8, backlog item #16).

`reclaim_orphaned_preparations(live_locators)` closes it. Every later state a
locator's record can be found in — `Active`, `PendingPublication`,
`PendingRotation` — requires that configuration write to have already
succeeded, so the decoded state alone proves whether a record is this leak's
orphan: a `PreparedInit` record whose locator `live_locators` does not name is
reclaimed; every other state is left alone unconditionally, regardless of
`live_locators`. Deletion is compare-and-delete against the exact revision
observed while listing, so a record that changes in between is left for
whatever legitimate write is racing it rather than torn out. See
`VLT-PM05-application.md` §7.3 for the full argument and `vault-pm-cli`'s
`begin_init`/`vault_create` for the two call sites.

## Verification

Eighteen tests cover generation-zero installation, rotation, idempotent retry,
stale and competing predecessors, malformed generation/pointer data,
supersession of a retired generation and refusal of the live one, exact
local-state CAS, concurrent writers, closed errors, filesystem reconstruction,
and reclaiming a generation zero orphaned before configuration — including a
record still live in the caller's configuration, an `Active` record left
untouched regardless of `live_locators`, a record this store never wrote, and
survival across reopening a real filesystem-backed store.

```bash
bash BUILD
cargo clippy -p coding_adventures_vault_pm_application_storage_core --all-targets -- -D warnings
```
