# `coding_adventures_vault_engine_kv2` — VLT08 KV-v2 engine

Versioned static KV — the workhorse engine of every Vault
deployment that holds long-lived shared credentials, configuration
secrets, or anything else that doesn't have a natural mint-on-
demand source. Also the on-ramp for password-manager-class
products: a Bitwarden / 1Password vault is structurally a single-
tenant KV-v2 mount with a typed-record codec on top (VLT02).

Implements [`SecretEngine`](../vault-engine-core/) from the trait
crate.

## Quick example

```rust
use coding_adventures_vault_engine_core::{
    MintContext, Role, SecretEngine,
};
use coding_adventures_vault_engine_kv2::{KvV2Config, KvV2Engine};
use coding_adventures_zeroize::Zeroizing;

let engine = KvV2Engine::new(KvV2Config::default())?;

// Per-call input rides on MintContext — no shared "staged"
// slot, so concurrent callers cannot cross streams.
let minted = engine.mint(
    &Role::new("shared"),
    &MintContext {
        principal: "alice".into(),
        now_ms: 0,                 // caller-supplied; engine is pure
        requested_ttl_ms: 60_000,
        path: Some("shared/db-password".into()),
        input: Some(Zeroizing::new(b"hunter2".to_vec())),
        cas_token: None,
    },
)?;

// Hand `minted.body` to a LeaseManager from VLT07, or:
let bytes = engine.read_latest("shared/db-password")?;
assert_eq!(&*bytes, b"hunter2");
```

## Semantics — same as HashiCorp Vault's KV-v2

| Operation                | Behaviour                                                   |
|--------------------------|-------------------------------------------------------------|
| `mint` (single call)     | new monotonic version; CAS via `ctx.cas_token`              |
| `cas_token: None`        | unconditional write                                         |
| `cas_token: Some(0)`     | create-only (path must have no live version)                |
| `cas_token: Some(N)`     | update from version N (current latest-live must equal N)    |
| `revoke(SecretRef::KvV2)`| soft delete: marks version destroyed, scrubs the bytes      |
| `read_latest`            | returns latest live version                                 |
| `read_version(N)`        | returns specific version (404 if destroyed)                 |
| `rotate_root`            | bumps engine generation counter                             |

`revoke` is **idempotent** — re-revoking a destroyed version is
not an error.

## TTL clamping

Effective TTL = `min(caller_requested_or_default, role.max_ttl_ms,
engine.max_ttl_ms)`. A zero requested TTL falls back to the
role default, then the engine default. A clamped-to-zero TTL is
rejected as `EngineError::InvalidParameter`.

## Threat model

- **In-memory only**: this reference implementation does not
  encrypt at rest. A storage-backed sibling crate will route
  through VLT01 sealed-store. The bytes are still held under
  `Zeroizing<Vec<u8>>` so they're scrubbed on revoke / cap-eviction
  / drop.
- **CAS, not locks**: concurrent writers to the same path race
  against the latest-live version; the loser sees `Conflict`.
- **No shared staging slot**: each `mint` call carries its own
  `path` / `input` / `cas_token` on `MintContext`. Two concurrent
  mints from different callers cannot cross streams (a
  confused-deputy hazard avoided by construction).
- **Soft delete**: revoked versions are kept in the table as
  zero-byte tombstones so the audit log can answer "this
  version existed and was destroyed at time T". `max_versions`
  caps *live* rows only; tombstones are not evicted.
- **Caller-supplied `now_ms`**: the engine never reads the
  system clock, keeping it deterministic for testing and
  capability-light.

## What this crate is not

- Not a typed-record codec — see `vault-records` (VLT02). KV-v2
  carries opaque bytes; a Bitwarden-style product layers
  typed-record framing on top.
- Not an encryption layer — see `vault-sealed-store` (VLT01).
- Not a transport — see `vault-transport-*` (VLT11).
- Not a storage backend — for that, a future sibling crate will
  store rows through `storage-core::StorageBackend`.

## Capabilities

None — pure in-memory. See `required_capabilities.json`.

See [`VLT00-vault-roadmap.md`](../../../specs/VLT00-vault-roadmap.md)
and [`VLT08-vault-dynamic-secrets.md`](../../../specs/VLT08-vault-dynamic-secrets.md).
