# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- **`put`'s parent-directory `fsync` failure is no longer discarded (backlog
  item #15).** It used to be `let _ = d.sync_all()` — any failure was
  silently swallowed, while `vault-pm-local-host` hard-fails the identical
  operation for `vault-pm.toml`. That asymmetry was flagged as an open
  question in VLT-PM41 §8.1: the owner-state records this crate persists
  carry the same crash-safety requirement as `vault-pm.toml` (both are read
  back by recovery code that assumes a durable `Ok`), so there was no
  argument for tolerating a failure here that the config file refuses to
  tolerate. On Unix, a failure now propagates as `StorageError::Unavailable`
  — the same error class the tmp-file `fsync` already uses. On Windows it
  stays a no-op, but for a structural reason rather than neglect:
  `std::fs::File::open` cannot open a directory handle there at all inside a
  `#![forbid(unsafe_code)]` crate, so there is nothing to attempt. See
  `code/specs/STR01-storage-fs-backend.md`, "Directory-fsync durability," for
  the full argument.
- **`delete` now fsyncs its parent directory too (backlog item #19).**
  Previously `delete` performed no fsync of any kind — a crash right after a
  successful delete could resurrect the file. This is defense-in-depth, not
  a fix for an open vulnerability: the one HIGH finding that depended on
  delete durability (the retired passphrase-rotation wrap,
  `vault-pm-application-storage-core::supersede_generation`) was already
  closed independently, by overwriting the sensitive body through the
  fsync-durable `put` path before ever calling `delete`. This change gives
  every other caller of `delete` the same durability `put` already had, using
  the identical `fsync_parent_directory` helper and identical hard-fail
  policy.
- Stopped `initialize()` re-issuing a revision that had already been handed out.
  The recovery scan ended in `revision_counter.store(highest)` performed WITHOUT
  `write_lock`, the lock `put()` holds while allocating revisions via
  `fetch_add`. Any later `initialize()` therefore reset the counter to whatever
  the on-disk high-water mark happened to be — which moves BACKWARDS as soon as
  the record holding that mark is deleted. The same revision is then issued
  twice, and a stale `if_revision` compare-and-swap guard passes where it must
  fail. The scan now runs once, under `write_lock`, behind a double-checked
  `scanned` flag.

  Recorded as theoretical in #12139 ("not currently known to be reachable"); it
  is in fact reachable deterministically and without any concurrency, by
  deleting the highest-revisioned record and calling `initialize()` again.
  `reinitialize_does_not_reissue_a_revision` is that sequence, and it fails on
  the pre-fix code with "re-initialize reissued revision 2, already held by k2".

- Stopped `initialize()` re-reading the entire state directory on every call.
  It read the body of every record in every namespace, and `ServiceRegistry`'s
  `load` and `list` both call it, so the walk ran several times per reconcile
  tick — over a directory shared by the registry, audit log, channel state and
  smart-home state, i.e. one that grows with data an agent can influence.
  Measured at ~480 ms per call against a 473 ms baseline tick (#12139).

- Made `initialize()`'s recovery walk total. Every failure was swallowed by an
  `if let Ok(..)`, which was survivable only while the scan re-ran on the next
  call: a transient `EACCES`/`EMFILE`/`EIO` produced `highest = 0` and the
  following tick corrected it. Cached, a swallowed error freezes the counter at
  a bogus floor for the life of the backend, and every revision it then issues
  is one already handed out. Three distinct swallows are closed:

  - `fs::read_dir` failures on the root and on each namespace.
  - `Path::is_dir()`, which answers `false` for ANY stat failure, so a root that
    was listable but not traversable made every namespace look like "not a
    directory" and skipped the entire store while returning `Ok`.
  - I/O failures reading a record. These matter most, because they fail in
    CORRELATED ways: under fd exhaustion every record read fails while the
    directory reads still succeed, collapsing the floor to zero.

  The tolerance that remains is drawn on error KIND, not on records versus
  directories: an unparseable record (`StorageError::Backend`) is a local fact
  and is skipped, because refusing to start over one corrupt file is the failure
  mode #12137 is about, while an unreadable one (`StorageError::Unavailable`)
  means "I could not tell" and propagates.

- Kept the walk O(1) in memory. Record counts are unbounded — `validate_path_like`
  caps a key's character set and shape, never how many keys exist — so the walk
  streams its directory iterators rather than collecting paths.

- Made the counter seed monotone (`fetch_max`, not `store`). `put()` does not
  require `initialize()`, so a first `initialize()` may legitimately run after
  revisions have been issued, and a plain `store` dropped the counter back below
  them. This alone would have prevented the original defect.

- Stopped a poisoned `write_lock` failing reads. `ServiceRegistry::{load,list,
  register}` all begin with `initialize()?`, so now that `initialize()` takes
  the write lock, treating poisoning as an error would take pure reads down
  along with writes. The guarded data is `()`, so the lock is recovered instead.

- Made parallel filesystem-backend tests allocate distinct temporary roots even
  when the platform clock returns the same timestamp.

- Made revisions unique across backend instances and across restarts, by
  deriving uniqueness from the instance rather than recovering it from disk.
  A revision is now `rev-<instance>-<counter>`.

  The previous scheme seeded the counter from the highest revision found on
  disk, which made the guarantee depend on those records still existing.
  Deleting the record holding the high-water mark moved it backwards, and the
  next instance reissued revisions already handed out. Two backends over one
  root — which `chief-of-staff-daemon` constructs — collided with no deletion at
  all, since both seeded from the same scan.

  A reissued revision is an ABA on every `if_revision` guard taken against it:
  a stale token matches a record it was never taken from, so a compare-and-swap
  that must fail silently succeeds. Structural uniqueness removes the class
  outright — deletion, a restored backup, a rolled-back store and a second
  concurrent process are all non-events, and there is no durable bookkeeping to
  lose or roll back.

  The instance identifier is 128 bits of OS entropy, drawn once per backend.
  An earlier draft derived it from process id, wall clock and a process-local
  counter, which does not hold: the counter resets in every new process, a
  container almost always starts its daemon as pid 1, `SystemTime` advances in
  microsecond steps rather than nanoseconds, and a pre-epoch clock collapsed the
  whole thing to a function of pid and a counter starting at zero — two runs as
  pid 1 then minted byte-identical revisions from the first `put`. Entropy has
  no ambient state to repeat and no platform resolution to depend on. A failed
  draw is reported rather than substituted for, because substituting silently is
  exactly how that draft went wrong.

  `storage_core` documents `Revision` as an **opaque** compare-and-swap token,
  so the new form deliberately does not sort. Nothing in the repo ordered them:
  `revision_to_u64` had no callers outside this crate (it is now deleted), and
  `vault-revisions` orders its own history by `archived_at_ms`.

- Removed revision recovery from `initialize()` entirely. The walk now sweeps
  stranded `.tmp` files and opens no record, so it costs O(directory entries)
  rather than O(bytes in the store) — on the 8 MB state directory #12139
  measured, the old walk was ~480 ms per call. The delicate reasoning about
  which read failures could be tolerated went away with the reads: there is no
  floor left to poison. Directory reads stay total, because the result is
  cached and a sweep that silently saw nothing would strand temporaries for good.

- Made the write lock per-ROOT rather than per-instance, so two backends over
  one directory actually exclude each other. `put` evaluates its
  `if_absent`/`if_revision` check against a read and only then renames; a
  per-instance mutex excluded nobody, so both backends passed the check and both
  wrote. `chief-of-staff-daemon` holds exactly that shape — two
  `FsStorageBackend`s over one state directory. A process-wide registry keyed by
  canonicalised root now hands both the same lock.

- Gave every write its own temporary. `<key>.tmp` was shared by all writers of
  that key, which is a silent-corruption primitive rather than untidiness: A
  writes the temporary, B truncates and rewrites it, A renames it into place,
  and the record then holds B's BYTES under A's REVISION — which A has already
  returned to its caller. Every `if_revision` guard later taken against that
  revision protects content it was never derived from. Reproduced at 1/40 rounds
  with 4 MiB bodies before the fix. Temporaries are now qualified by instance
  and a per-write counter, so a rename can only commit its own writer's bytes.

### Known limitation

- **Cross-process write exclusion is still missing.** The lock above is
  process-wide, not system-wide, so two *processes* over one root can still both
  pass a CAS check and both rename. Closing it needs an `flock`/`O_EXCL` lock
  spanning check-through-rename, which this crate cannot express while it is
  `#![forbid(unsafe_code)]` — the existing in-repo lock
  (`vault-pm-local-host`) uses `libc::openat`/`CreateFileW`, and it sits *above*
  this crate rather than below it.

  Scope honestly: the per-write temporary reduces the cross-process failure from
  silent corruption to a lost update, which is a real improvement but not
  exclusion. That reduction is **not proven by the tests here** — they run
  in-process, where the new lock already covers it.

  Until a system-wide lock lands, any protocol whose safety rests on CAS-based
  mutual exclusion between processes — D18R leadership fencing, for one — is not
  sound on this backend and should not assume an atomic compare-and-swap.

### Added

- Atomic create-if-absent writes under the backend write lock, verified through
  the shared `storage-core` conformance suite.
- `FsStorageBackendSummary`, `fs_storage_backend_summary()`, and
  `FsStorageBackend::surface_summary()` for payload-free inspection of
  STR-FILE record format, crash-safety, ciphertext opacity, lease, and
  cross-process locking guarantees.

## [0.1.0] — 2026-05-04

### Added

- Initial implementation of STR-FILE
  (`code/specs/STR01-storage-fs-backend.md`).
- `FsStorageBackend` — implements the
  `storage_core::StorageBackend` trait against a directory tree
  on disk.
- Disk layout: `<root>/<hex(namespace)>/<hex(key)>` per record.
  Hex-encoded names so arbitrary key bytes survive the
  filesystem's allowed-character rules.
- Single-file binary record format:
  `magic(4)"STRF" || version(1)=1 || meta_len(4 BE) || meta_json(N) || body(rest)`.
  `meta_json` carries `revision`, `content_type`, `created_at`,
  `updated_at`, and the caller-supplied JSON metadata.
- Atomic write + rename + fsync for crash safety:
  1. Write header + meta + body to `<key>.tmp`.
  2. `fsync` the tmp file.
  3. POSIX `rename(2)` to `<key>` (atomic vs concurrent readers).
  4. Best-effort `fsync` of the parent directory.
- `initialize()` walks `<root>`, removes stranded `.tmp` files,
  and seeds the in-memory revision counter from the highest
  revision found on disk so monotonic numbering survives
  process restart.
- In-memory advisory leases (same shape as
  `InMemoryStorageBackend`'s) — durable cross-process leases
  would require platform-specific `flock`/`lockf` and are
  deferred.
- Shared `storage-core` conformance coverage for initialize,
  put/get, stale compare-and-swap rejection, idempotent delete,
  prefix listing order, and lease expiry.
- 19 unit tests covering: put/get round-trip, missing-record
  read returns None, overwrite advances revision, CAS with
  correct/wrong/missing-record `if_revision`, delete +
  delete-missing + delete-with-wrong-revision-conflicts, list
  sorted by key, list with prefix filter, list of unknown
  namespace returns empty, stat returns metadata without body,
  initialize-removes-stranded-tmp-files, restart-picks-up-
  revision-counter (monotonic across `drop`+rebuild),
  acquire-lease first-time / held-returns-None, corrupted-magic
  → `Backend` error, truncated-file → `Backend` error.

### Out of scope (future PRs)

- Encryption — that's VLT01 sealed-store, layered above this.
- Replication / sync — VLT10.
- Cross-process advisory locks (POSIX `flock`/`lockf`).
- Cloud-backed implementations: S3 / GCS / Google Drive /
  WebDAV / git. Each is a sibling crate following the same
  `StorageBackend` trait. The crucial property — *the backend
  only ever sees ciphertext* — is preserved by them all because
  VLT01 sits above.
- Garbage-collection of orphaned namespace directories.
