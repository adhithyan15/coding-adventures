# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-05-04

### Added

- Initial implementation of VLT10
  (`code/specs/VLT10-vault-sync-engine.md`).
- `DeviceId` — opaque, ordered, hashable identifier with
  bounded length.
- `VersionVector` — `BTreeMap<DeviceId, u64>` with `bump`,
  `merge`, `compare`, `dominates`, `concurrent_with`.
  Comparison returns the four-valued `VectorOrdering`
  (`Equal` / `Dominates` / `DominatedBy` / `Concurrent`).
- `SyncRecord` — `(namespace, key, version_vector,
  last_writer, last_writer_ms, ciphertext, wrap_set)`. Ciphertext
  and wrap-set are opaque to the server. `validate()` enforces
  bounded sizes on every variable-length field.
- `SyncError` — `InvalidParameter` / `Server` / `NotFound`.
- `SyncServer` trait — `push` / `get` / `pull(namespace,
  since)`. `Send + Sync`, object-safe.
- `PushOutcome` — `Applied` / `Stale` / `ConflictResolved` /
  `Unchanged`. `ConflictResolved` returns both winner and loser
  so the UI can present a merge dialog.
- `LwwResolver` — last-writer-wins with deterministic
  device-id tie-break (smaller-id wins on equal timestamps).
  Winner's `version_vector` is the merge of both inputs.
- `OrSet` — observed-removal Set CRDT keyed by `String`.
  `add(value, device, now_ms)` / `remove(value)` /
  `contains(value)` / `values()` / `merge(&other)`. Idempotent,
  commutative, associative — verified by tests.
- `InMemorySyncServer` — reference implementation backed by a
  `Mutex<HashMap<(String, String), SyncRecord>>`.
- 38 unit tests covering: empty-vector equality,
  bumping-dominates, divergent-devices-concurrent,
  pointwise-max merge, merge-dominates-both,
  validate rejection on empty/oversized fields and
  vector-without-last-writer, DeviceId rejects empty/oversize,
  LWW picks higher timestamp, LWW tie-break by smaller
  device-id, first-push-applied,
  dominating-push-replaces-existing,
  dominated-push-returns-stale, idempotent-push-unchanged,
  concurrent-push-resolves-via-LWW,
  equal-vector-different-bytes-treated-as-conflict,
  pull-returns-unseen-only, pull-skips-other-namespaces,
  get-unknown-returns-NotFound, Send+Sync compile-time check,
  OrSet add/remove/readd-after-remove,
  OrSet merge unions concurrent adds (idempotent +
  commutative + associative), OrSet remove propagates via
  merge, 16-thread concurrent push.
- `#![forbid(unsafe_code)]` + `#![deny(missing_docs)]`.

### Security hardening (pre-merge review)

Five findings flagged before push, all fixed inline:

- **MEDIUM** — Equal-vector branch silently dropped wrap-set
  rotations: re-pushing the same `(vector, ciphertext, writer)`
  with a *different* `wrap_set` (e.g. VLT04 recipient list
  rotated) returned `Unchanged` while keeping the old recipient
  set on the server — a real availability + confidentiality
  hazard if a revoked recipient stayed in the canonical wrap-
  set. Fix: equality now requires `ciphertext + last_writer +
  last_writer_ms + wrap_set` to all match; any difference routes
  through `ConflictResolved` so the application can decide.
- **MEDIUM** — `OrSet` tag uniqueness gap: two independently-
  constructed `OrSet`s on the same device that each call
  `add("x")` produced the same `(device, 1)` tag. Merging
  collapsed both into one observation, and a remove on either
  side then tombstoned both. Fix: added `add_with_tag(value,
  device, tag_id, now_ms)` that takes a caller-supplied
  globally-unique tag id (natural source: the field's
  `VersionVector[device]`); `add()` is documented as
  single-instance-per-(device, field).
- **LOW** — Control chars and whitespace allowed in `DeviceId`,
  `namespace`, `key`: enabled log-injection / format-confusion
  in downstream layers. Now rejected by validate.
- **LOW** — `Debug` on `SyncRecord` dumped raw ciphertext +
  wrap_set bytes. Replaced with hand-rolled redacted form
  (lengths only).
- **LOW** — Mutex poisoning panicked the server permanently on
  any prior thread panic. Replaced `lock().expect()` with a
  `PoisonError::into_inner()`-based recovery helper.

Also tightened: `VersionVector::bump` now panics on `u64`
overflow (unreachable, but documents the contract); the
`SyncServer` trait docs now state explicitly that authorisation
gating is the upper layers' job and that wire-tier validation
must enforce caps before deserialisation.

### Out of scope (future PRs)

- **Persistent server** — `vault-sync-postgres`,
  `vault-sync-sqlite`, `vault-sync-s3`. Each implements the
  same `SyncServer` trait against its native storage.
- **Wire transport** — VLT11 transports route this layer over
  TLS / gRPC / HTTP.
- **Authenticated server** — VLT05/VLT06 above route auth +
  authorization for `push`/`pull` calls.
- **Attachment chunking** — VLT14 splits large payloads;
  ciphertext bound at 1 MiB per record here.
- **Tombstone GC** — currently retained indefinitely. Future
  work to safely garbage-collect once all known devices have
  observed a tombstone.
- **More CRDT types** — only `OrSet` for now. LWW-Register and
  PN-Counter are obvious next siblings.
