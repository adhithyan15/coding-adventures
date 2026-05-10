# VLT07 — Vault Lease Manager

## Overview

A **lease manager** hands out, refreshes, and reaps short-lived
opaque tokens. It is the building block underneath:

- HashiCorp Vault's *response wrapping* — one-shot capability
  tokens for secure introduction.
- VLT08 dynamic-secret engines — every mint of a fresh AWS / DB /
  PKI credential is wrapped in a lease so the consumer (and the
  audit log) know exactly when it dies.
- Recovery / invitation tokens — short-lived bearer creds for
  out-of-band hand-off in 1Password / Bitwarden style products.

Implementation lives at `code/packages/rust/vault-leases/`.

## Why a separate layer

VLT01..VLT06 cover *what* the secret is and *whether* a caller
may have it. VLT07 adds the orthogonal concern of *for how long*
and *how to take it back*. Leases are content-agnostic — the
manager treats the body bytes as opaque — and decision-engine
agnostic — the policy decision (VLT06) has already happened by
the time `issue()` is called.

Splitting it out keeps three downstream crates simpler:

1. **VLT08** consumes `LeaseManager` by trait object so a
   dynamic-secret engine doesn't care whether leases live in
   memory, in storage-core, or in Redis. Tests use the in-memory
   reference implementation.
2. **VLT09 (audit)** subscribes to lease lifecycle events
   externally; the lease manager itself stays silent so the
   dependency arrow is one-way.
3. **VLT10 (sync)** propagates revocations across replicas using
   the same trait — a `ReplicatedLeaseManager` is a wrapper, not
   a fork.

## Public API

```rust
pub struct LeaseId(/* 128-bit CSPRNG, hex-encoded */);
pub struct LeasePayload(/* Zeroizing<Vec<u8>> */);
pub struct LeaseInfo {
    pub id: LeaseId,
    pub issued_at_ms: u64,
    pub expires_at_ms: u64,
    pub read_count: u32,
    pub revoked: bool,
}

pub enum LeaseError {
    NotFound,
    Expired,
    Revoked,
    InvalidParameter(&'static str),
    Crypto(coding_adventures_csprng::CsprngError),
}

pub trait LeaseManager: Send + Sync {
    fn issue(&self, payload: LeasePayload, ttl_ms: u64) -> Result<LeaseId, LeaseError>;
    fn renew(&self, id: &LeaseId, extra_ms: u64) -> Result<(), LeaseError>;
    fn revoke(&self, id: &LeaseId) -> Result<(), LeaseError>;
    fn lookup(&self, id: &LeaseId) -> Result<LeaseInfo, LeaseError>;
    fn read(&self, id: &LeaseId) -> Result<LeasePayload, LeaseError>;
    fn consume(&self, id: &LeaseId) -> Result<LeasePayload, LeaseError>;
    fn expire_due(&self, now_ms: u64) -> Result<usize, LeaseError>;
}

pub struct InMemoryLeaseManager;
```

## Semantics

| Method        | Invariants                                                           |
|---------------|----------------------------------------------------------------------|
| `issue`       | `ttl_ms > 0`. Returns a fresh ID; collision → `InvalidParameter`.    |
| `renew`       | New expiry = `now + extra_ms`. Never extends from existing expiry.   |
| `revoke`      | Idempotent. Payload is dropped (and zeroized) immediately.           |
| `lookup`      | Always returns `LeaseInfo` if the entry is still in the table —     |
|               | even for revoked or expired leases — so callers can introspect.     |
| `read`        | Returns a fresh `Zeroizing<Vec<u8>>` clone; bumps `read_count`.      |
| `consume`     | Atomic read-and-revoke: holds the mutex for the whole operation.    |
| `expire_due`  | Caller supplies `now_ms`. Reaps revoked + `expires_at_ms <= now_ms`. |

## On-disk / persistence

The reference implementation is purely in-memory. A
storage-backed implementation will arrive as a sibling crate
(e.g. `coding_adventures_vault_leases_storage`) following the
same trait, with payloads encrypted by VLT01 sealed-store before
they reach storage-core.

## Threat model & test coverage

| Threat                                                         | Defence                                                         | Test                                                                |
|----------------------------------------------------------------|-----------------------------------------------------------------|---------------------------------------------------------------------|
| Lease ID guessable                                             | 128-bit CSPRNG, hex-encoded                                     | `distinct_ids_for_distinct_issues`                                  |
| Existence oracle via wrong-ID timing                           | Uniform `NotFound` for never-issued vs reaped                   | `read_unknown_id_returns_not_found`, `revoke_unknown_id_is_not_found` |
| Replay after consume                                           | `consume` revokes atomically under one mutex acquisition        | `consume_returns_payload_and_revokes_atomically`                    |
| Sleeping consumer banks renewals                               | `renew` resets expiry from *now*, not existing expiry           | `renew_extends_expiry`                                              |
| Renewing a dead lease silently succeeds                        | `renew` checks revoked + expired before mutating                | `renew_revoked_returns_revoked`, `renew_after_natural_expiry_returns_expired` |
| Reading after natural expiry returns stale bytes               | `read` checks `expires_at_ms <= now`                            | `read_after_natural_expiry_returns_expired`                         |
| Consume on expired lease leaks bytes                           | `consume` checks expiry before taking payload                   | `consume_after_natural_expiry_returns_expired`                      |
| Payload bytes linger in memory after revoke                    | `revoke` drops the `Zeroizing<Vec<u8>>` immediately             | structurally enforced by `LeaseEntry::Drop` calling `zeroize`       |
| `dbg!(payload)` leaks the body                                 | `Debug` for `LeasePayload` is hand-rolled, body redacted        | structurally — the `Debug` impl never reads the bytes               |
| Caller passes zero TTL by accident                             | `issue` rejects `ttl_ms == 0`; `renew` rejects `extra_ms == 0`  | `issue_with_zero_ttl_rejected`, `renew_with_zero_extra_rejected`    |
| Unbounded TTL pins entry past any sweep                        | `issue`/`renew` reject `ttl_ms > MAX_TTL_MS = 90d`; `checked_add` | `issue_rejects_ttl_above_max`, `renew_rejects_extra_above_max`    |
| TOCTOU on clock read widens expiry-edge window                 | `now_ms()` sampled *under* the mutex in `issue`/`renew`/`read`/`consume` | structural — single mutex acquisition holds across read+check     |
| Bearer-cap residue in heap after reap                          | `LeaseId` inner string held under `Zeroizing<String>`           | structural — `LeaseEntry::Drop` runs `Zeroize` and HashMap key drops the `Zeroizing` |
| Bearer-cap leak via `dbg!(lease_id)`                           | Hand-rolled redacted `Debug` on `LeaseId` (mirrors `LeasePayload`) | structural — the impl never reads the bytes                       |
| Malformed external ID surfaces as `NotFound`                   | `LeaseId::from_hex` rejects non-32-char or non-lowercase-hex    | `lease_id_from_hex_rejects_bad_input`                               |
| Background sweeper has uncontrolled side effect                | No background thread; `expire_due` is caller-driven             | `expire_due_removes_old_and_revoked`                                |

## Out of scope (future PRs)

- **Encryption at rest** — VLT01 sealed-store sits underneath a
  storage-backed implementation; the in-memory reference doesn't
  need it.
- **Audit log** — VLT09; the lease manager intentionally stays
  silent.
- **Replicated revocation** — VLT10 sync engine.
- **Persistent backend** — `StorageBackedLeaseManager` is a thin
  shim over `storage_core::StorageBackend`.
- **Background sweep timer** — keeping the crate a pure library
  means no async runtime, no thread, no timer wheel.
- **Quorum-revocable leases** (M-of-N must agree to revoke) — a
  decorator that wraps `LeaseManager`, future work.

## Citations

- HashiCorp Vault Lease & Renew API — semantic reference.
- HashiCorp Vault Response Wrapping — the `consume()` primitive
  is the same shape.
- VLT00-vault-roadmap.md — VLT07 placement.
- VLT08-vault-dynamic-secrets.md (forthcoming) — the largest
  consumer of this trait.
- `coding_adventures_csprng::random_array` — ID entropy source.
- `coding_adventures_zeroize::Zeroizing` — payload wiping.
