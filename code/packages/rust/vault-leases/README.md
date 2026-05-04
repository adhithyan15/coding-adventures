# `coding_adventures_vault_leases` — VLT07

Lease manager for the Vault stack. Issue, renew, revoke, look up,
and consume short-lived capability tokens over arbitrary opaque
payloads.

This is the building block behind:

- **HashiCorp Vault response wrapping** — one-shot pointer to a
  secret, atomic `consume()` revokes after the first read.
- **Dynamic-secret leases** (VLT08) — fresh AWS / database / PKI
  credentials handed out with a TTL, automatically reaped.
- **Recovery / invitation tokens** — short-lived bearer creds for
  out-of-band hand-off.

## Quick example

```rust
use coding_adventures_vault_leases::{
    InMemoryLeaseManager, LeaseManager, LeasePayload,
};

let mgr = InMemoryLeaseManager::new();

// Wrap a temporary credential pair under a 30-minute lease.
let body = b"{\"access_key\":\"AKIA...\",\"secret_key\":\"...\"}".to_vec();
let id = mgr.issue(LeasePayload::new(body), 30 * 60 * 1_000)?;

// Hand `id.as_hex()` to the consumer over a side channel.

// Consumer redeems exactly once. After this, any further use of
// the same ID returns `LeaseError::Revoked`.
let payload = mgr.consume(&id)?;
process(payload.as_bytes());
```

## API at a glance

| Method        | Purpose                                                  |
|---------------|----------------------------------------------------------|
| `issue`       | mint a fresh ID and stash a payload under a TTL          |
| `renew`       | extend the TTL by `extra_ms` from *now*                  |
| `revoke`      | mark the lease dead (idempotent)                         |
| `lookup`      | read metadata only (no payload bytes)                    |
| `read`        | multi-read: returns payload + bumps `read_count`         |
| `consume`     | one-shot: returns payload **and** revokes atomically     |
| `expire_due`  | sweep entries whose expiry is `<= now_ms` or are revoked |

## Where it fits

```text
                 ┌──────────────────────────────────────┐
                 │  VLT08 dynamic-secret engines        │
                 │  (AWS / DB / PKI credential mints)   │
                 └──────────────┬───────────────────────┘
                                │ wraps minted creds in
                                │ a TTL'd lease
                 ┌──────────────▼───────────────────────┐
                 │  VLT07 lease manager (THIS CRATE)    │
                 │  issue / renew / revoke / consume    │
                 │  read / lookup / expire_due          │
                 └──────────────┬───────────────────────┘
                                │ payload bytes
                 ┌──────────────▼───────────────────────┐
                 │  zeroize::Zeroizing — payload wiping │
                 │  csprng — 128-bit random IDs         │
                 └──────────────────────────────────────┘
```

## Threat model

- **ID guessability**: 128-bit CSPRNG-drawn, hex-encoded.
- **`NotFound` is uniform**: indistinguishable for "never issued"
  vs "already reaped" — no oracle.
- **Payload zeroization**: every payload is held in
  `Zeroizing<Vec<u8>>` and wiped on revoke / expire / drop.
- **Atomic `consume`**: mutex-guarded read-and-revoke; a second
  `consume` always sees `Revoked`.
- **`Debug` on payload is redacted** so a stray `dbg!` cannot leak
  the bytes.

## What this crate is not

- Not encrypted at rest. Layer VLT01 sealed-store under a
  storage-backed implementation when you need that.
- Not replicated. Cross-replica revocation propagation is VLT10.
- Not audited. VLT09 audit log subscribes to lease events; this
  crate stays silent so the dependency arrow is one-way.
- Not a job queue or KV store — see `storage_core::StorageBackend`.

## Capabilities

`csprng` (for ID minting) and `wallclock` (for TTL bookkeeping).
See `required_capabilities.json`.
