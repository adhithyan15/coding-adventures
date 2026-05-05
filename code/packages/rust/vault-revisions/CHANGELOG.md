# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-05-04

### Added

- Initial implementation of VLT12
  (`code/specs/VLT12-vault-revision-history.md`).
- `Revision` — `(id, archived_at_ms, ciphertext: Vec<u8>)`.
  Hand-rolled `Debug` redacts the ciphertext (lengths only).
- `RevisionMeta` — read-only metadata view used by `list()`.
  Carrying it instead of the full `Revision` lets the common
  "what versions exist?" call avoid copying the bytes.
- `RetentionPolicy` — per-namespace `max_revisions_per_key` +
  `max_age_ms`, both `Option<…>` so either can be unbounded.
  `RetentionPolicy::default_password_manager()` returns 32
  revisions / 90 days; `::unbounded()` keeps everything.
- `RevisionError` — narrow variants:
  `InvalidParameter` / `UnknownRevision` / `NotFound` /
  `Backend` / `Overflow`. `Display` impl never echoes
  caller-supplied bytes.
- `RevisionStore` trait — `Send + Sync`, object-safe:
  `archive` / `list` / `get_revision` / `restore` /
  `purge_due` / `policy_for` / `set_policy`.
- `InMemoryRevisionStore` — reference implementation. Uses
  `BTreeMap<(String,String), History>` so iteration order is
  deterministic. Mutex poisoning is recovered via
  `PoisonError::into_inner()` (consistent with `vault-audit` /
  `vault-sync`).
- 30 unit tests covering: archive returns id 1, second archive
  increments, list returns metadata only (no ciphertext), list
  on unknown path returns empty, get_revision returns
  ciphertext + correct error variants for unknown id /
  unknown path, restore appends new revision (not in-place),
  restore on unknown revision errors, retention policy:
  max_revisions evicts oldest first, purge_due evicts old rows,
  purge_due is no-op without max_age, purge_due is namespace-
  scoped, default policy caps after archive, unbounded policy
  keeps everything, validation rejects empty/oversize
  namespace + key + ciphertext, control-char rejection in
  namespace, bidi-override rejection in key (CVE-2021-42574),
  empty / oversize ciphertext, Revision `Debug` redacts bytes,
  Send+Sync compile-time check, 16-thread concurrent archive
  produces dense ids 1..=64, set_policy round-trips.

### Security hardening (pre-merge review)

Three findings flagged before push, all fixed inline:

- **LOW** — `set_policy` and `policy_for` accepted unvalidated
  namespace strings (empty / oversize / control chars / bidi
  override), bypassing the rest of the crate's validation
  posture and creating an unbounded HashMap-key surface. Fix:
  `set_policy` now returns `Result<(), RevisionError>` and
  validates the namespace via the same helper as `archive`.
  `policy_for` stays infallible and falls back to the default
  on unknown namespaces (safe because `set_policy` validates
  on the way in).
- **LOW** — `set_policy` accepted
  `max_revisions_per_key = Some(0)`, which silently created a
  "history is lost" footgun where every archive returned
  successfully but immediately evicted the row. Fix:
  rejected at the `set_policy` boundary with
  `InvalidParameter`. Use `None` to disable.
- **LOW** — `purge_due` validated namespace charset / non-
  emptiness inline but not length cap, an inconsistency with
  `archive` / `list`. Fix: switched to the shared
  `validate_namespace` helper. Also documented that `now_ms`
  is privileged input that the host MUST source from a
  trusted clock (a `u64::MAX` value would correctly purge
  everything — by design for an admin who genuinely wants
  that).

### Bounds

- `MAX_NAMESPACE_LEN = 128`, `MAX_KEY_LEN = 512`, `MAX_CIPHERTEXT_LEN = 1 MiB`.

### Out of scope (future PRs)

- **Persistent stores** — `vault-revisions-fs`,
  `vault-revisions-postgres`. Each implements the same
  `RevisionStore` trait against its native storage.
- **Cross-tier hash chaining** — VLT09 audit log records the
  archive events; this crate stays silent.
- **Re-wrap on restore** — applications that rotate keys may
  want `restore()` to re-encrypt under the current key. This
  crate hands back the historical ciphertext as-is; re-wrap
  belongs in the application or a thin VLT01-aware adapter.
- **Garbage collection of orphaned histories** — destruction of
  a key's history is gated by VLT06 in a future PR.
- **Index for time-range / actor queries** — VLT13 encrypted
  search will provide the typed index.
