# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-05-04

### Added

- Initial implementation of the KV-v2 secret engine
  (`code/specs/VLT08-vault-dynamic-secrets.md`).
- `KvV2Engine` — implements `SecretEngine` from
  `coding_adventures_vault_engine_core`.
- `KvV2Config` — `mount_path`, `max_versions` (default 16),
  `max_ttl_ms` (default 24h), `default_ttl_ms` (default 1h).
- `KvV2Engine::new(cfg) -> Result<Self, EngineError>` — empty
  mount path returns `EngineError::InvalidParameter` instead of
  panicking, so request-handling code that constructs engines
  from untrusted input cannot be made to panic.
- `mint(role, ctx)` — reads `path`, `input`, and `cas_token`
  directly from `MintContext` (no shared "staged write" slot;
  every call is self-contained). Allocates the next monotonic
  version under a single mutex acquisition and returns
  `MintedSecret { body, secret_ref: SecretRef::KvV2 { path, version }, granted_ttl_ms }`.
  Bytes are wrapped in `Zeroizing` directly (no bare-Vec
  intermediates).
- CAS modes via `ctx.cas_token`: `None` unconditional, `Some(0)`
  create-only, `Some(N>0)` update-from-N. `cas_token` values
  that don't fit in `u32` return `InvalidParameter`.
- `revoke(SecretRef::KvV2)` — soft-delete: marks the version
  `destroyed` and replaces the body with an empty
  `Zeroizing<Vec<u8>>`. Idempotent. Returns `UnknownSecret` for
  unknown paths/versions or for a `SecretRef` of a different
  variant.
- `rotate_root` — bumps an engine-level generation counter so
  audit-log consumers can correlate before/after. (A
  storage-backed sibling crate will rewrap rows under a new
  DEK.)
- `read_latest(path)` / `read_version(path, version)` —
  zeroizing-clone reads. Destroyed versions return
  `UnknownSecret`.
- TTL clamping: `granted = min(requested_or_default,
  role.max_ttl_ms, engine.max_ttl_ms)`. Zero-after-clamp is
  rejected.
- `max_versions` cap: applies to *live* rows only. Tombstones
  (soft-deleted versions) are kept indefinitely so the audit log
  retains its "this version existed and was destroyed" record;
  they cost ~24 B each and have an empty `Zeroizing<Vec<u8>>`
  body. Eviction removes the oldest *live* row when the live
  count exceeds the cap.
- 28 unit tests covering: mount-path round-trip, empty-mount
  returns `InvalidParameter`, mint round-trip, version
  monotonicity, read-latest, read-version, unknown-path →
  UnknownSecret, all three CAS modes
  (create-only-rejects-overwrite, update-from-wrong-version
  rejected, update-from-correct-version accepted),
  cas_token-overflows-u32 rejected, mint without
  path/input/empty-path/empty-input each return
  `InvalidParameter`, revoke-makes-version-unreadable, revoke
  idempotency, revoke on unknown / wrong-variant ref,
  revoke-then-latest fallback to prior live, TTL clamping
  (engine + role), zero-request → default TTL, rotate_root
  repeatable, max_versions caps live history while keeping
  tombstones, Send+Sync compile-time check,
  `into_lease_payload` after mint, and a 16-thread concurrent
  mint test that proves cross-stream confusion is structurally
  impossible.

### Security hardening (pre-merge review)

Three findings flagged before push, all fixed inline:

- **HIGH — Stage/mint cross-caller race**: dropped the
  single-slot `staged: Option<StagedWrite>` entirely. Per-call
  inputs (`path`, `input`, `cas_token`) now ride on
  `MintContext`. Concurrent callers cannot clobber each other's
  staged data because there is no shared staging slot.
- **MEDIUM — Non-zeroizing `Vec<u8>` intermediates**: bytes are
  now wrapped in `Zeroizing` immediately on creation in `mint`.
- **LOW — `max_versions` evicts tombstones, eroding audit
  trail**: eviction now counts only *live* rows; destroyed
  tombstones are kept so the README's audit-log claim holds.
- `#![forbid(unsafe_code)]` + `#![deny(missing_docs)]`.
