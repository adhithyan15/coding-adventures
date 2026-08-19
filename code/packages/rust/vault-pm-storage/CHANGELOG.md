# Changelog

All notable changes to this package are documented here.

## Unreleased

### Added

- `ReplicaSetObjectStore<P, M>`, VLT-PM00 §11.5's mirror decorator: publishes
  every accepted `put_immutable` to a configured set of mirror stores after
  the primary commit succeeds, never before and never gating it (§19.2).
  `initialize` and `put_immutable` are best-effort on each mirror; `get` falls
  back to mirrors when the primary reports the object missing or itself
  errors. `list`, `stat`, `delete_unreferenced`, and `changes` are
  primary-only in this slice.
- `ReplicaHealth`, a per-mirror attempted/succeeded/last-error snapshot
  exposed by `replica_health()`, so a caller (`storage check`) can report a
  degraded replica from an observed failure rather than a guess.
- `ReplicaSetObjectStore::single`, the zero-mirror construction used wherever
  a caller does not need replication; passes the same 24-check conformance
  suite as the store it wraps, unmodified.

### Deferred

- The explicit `sync --wait` ceremony and its configurable `one`/`all`/quorum
  durability target (§19.2) — this slice's mirror writes are unconditionally
  best-effort and asynchronous-in-spirit-but-synchronous-in-implementation
  (no network I/O lives in this crate; a real network-backed mirror adapter
  would need its own timeout policy).
- Physical-delete propagation to mirrors, left to a future replica-aware GC
  planner (§19.4) so a mirror never loses a still-referenced object ahead of
  every device observing the pruning checkpoint.

## [0.1.0] - 2026-08-09

### Added

- The bounded, provider-neutral `VaultObjectStore` V1 contract.
- Redacted identifiers, bodies, cursors, provider revisions, and typed errors.
- Capability reporting for consistency, conditional operations, change feeds,
  checksums, upload/read optimizations, deletion, sharing, and provider limits.
- A thread-safe deterministic in-memory backend with immutable writes, exact
  reads/stats, ordered cursor pagination, deletion, and change hints.
- A one-shot deterministic fault wrapper for provider errors, corrupt reads,
  stale or duplicate listings, and ambiguous committed writes.
- A reusable conformance runner and embedded language-neutral fixture.

### Security

- Conflicting bytes under one logical object ID fail as corruption.
- Store instances bind idempotently to exactly one vault locator.
- Debug and display output omit bodies, identifiers, cursors, revisions, and
  attacker-controlled provider messages.
- All V1 body, cursor, revision, list, and change-page bounds are explicit.
