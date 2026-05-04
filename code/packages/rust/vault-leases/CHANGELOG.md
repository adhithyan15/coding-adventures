# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-05-04

### Added

- Initial implementation of VLT07
  (`code/specs/VLT07-vault-leases.md`).
- `LeaseId` — opaque newtype wrapping a 32-char lowercase-hex
  string drawn from 128 bits of CSPRNG output. Strict
  `from_hex` rejects malformed input at the boundary instead of
  bouncing back as a confusing `NotFound`.
- `LeasePayload` — owned bytes wrapped in
  `coding_adventures_zeroize::Zeroizing<Vec<u8>>`. `Debug` is
  hand-rolled "redacted" so a stray `dbg!` cannot leak the body.
- `LeaseInfo` — read-only metadata view (`id`, `issued_at_ms`,
  `expires_at_ms`, `read_count`, `revoked`). Deliberately does
  *not* carry the payload — callers opt explicitly into
  `read()` / `consume()` to obtain it.
- `LeaseError` — narrow variants:
  `NotFound` / `Expired` / `Revoked` / `InvalidParameter` /
  `Crypto`. `NotFound` is uniform for never-issued and
  already-reaped IDs (no existence oracle).
- `LeaseManager` trait — `Send + Sync`, surface for `issue` /
  `renew` / `revoke` / `lookup` / `read` / `consume` /
  `expire_due`.
- `InMemoryLeaseManager` — reference implementation.
  - `issue` mints a 128-bit ID, stashes the payload, sets TTL.
  - `renew` extends from *now*, never from existing expiry, so a
    sleeping consumer can't bank renewals.
  - `revoke` is idempotent; payload is dropped (and zeroized)
    immediately, but the metadata row stays so the next call sees
    `Revoked` rather than `NotFound`.
  - `read` increments `read_count`, returns a fresh
    `Zeroizing` clone of the payload.
  - `consume` is atomic read-and-revoke under a single mutex
    acquisition — the basis for response wrapping.
  - `expire_due(now_ms)` is caller-driven; the manager has no
    background timer so the crate stays a pure library.
  - `LeaseEntry` implements `Zeroize` + `Drop` so reaped rows are
    scrubbed from memory.
- 21 unit tests covering: issue+lookup, zero-TTL rejection,
  TTL-above-max rejection, renew-extra-above-max rejection,
  multi-read counter increment, NotFound for unknown IDs,
  atomic consume + double-consume → Revoked, revoke makes read
  fail, idempotent revoke, NotFound for revoking unknown ID,
  renew extends expiry, renew-revoked → Revoked, zero-extra
  rejected, expire_due reaps both old and revoked rows,
  hex round-trip, hex parser strictness, 256 distinct IDs (no
  collision), payload Clone independence, Send+Sync compile-time
  check, and the three "natural-expiry" paths (read / renew /
  consume after wall-clock expiry).

### Security hardening (pre-merge review)

- **TOCTOU on clock reads**: `now_ms()` is now sampled *under* the
  mutex in `issue` / `renew` / `read` / `consume` so the expiry
  comparison is consistent with concurrent operations. Previously
  the clock was read before the lock was acquired; under
  contention this widened the window during which an
  already-expired lease could be acted on.
- **Bounded TTL**: `issue` and `renew` reject `ttl_ms` /
  `extra_ms` greater than `MAX_TTL_MS = 90 days`. The previous
  `saturating_add` would have pinned a lease to effectively
  infinite expiry on a `u64::MAX` input, growing the in-memory
  table without bound. Replaced with `checked_add` + the
  upper-bound check.
- **`LeaseId` is zeroized on drop**: the inner hex string is now
  held under `Zeroizing<String>` so reaped IDs are scrubbed from
  the heap. IDs are bearer capabilities; without zeroization the
  freed allocation would still contain the 32-char hex form
  until reused by the allocator. `Debug` on `LeaseId` is
  redacted (matches `LeasePayload`) so a stray `dbg!` cannot
  externalise the bearer cap.

### Out of scope (future PRs)

- Encryption at rest — layer VLT01 sealed-store under a
  storage-backed implementation.
- Replicated revocation — VLT10 sync engine.
- Audit logging of lease events — VLT09.
- Persistent backend (storage-core + VLT01).
- Periodic background sweeps — `expire_due(now_ms)` is
  caller-driven by design.
