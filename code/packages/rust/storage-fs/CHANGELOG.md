# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

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

- Made parallel filesystem-backend tests allocate distinct temporary roots even
  when the platform clock returns the same timestamp.

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
