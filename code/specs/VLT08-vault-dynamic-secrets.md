# VLT08 — Vault Dynamic-Secret Engines

## Overview

A *secret engine* is a plugin that produces secrets on demand
(instead of holding them at rest). Issuing a secret means: run
the engine's `mint` operation, wrap the result in a lease (VLT07),
return.

This spec covers two crates that ship together as the first
slice of the dynamic-secret tier:

- `coding_adventures_vault_engine_core` — the `SecretEngine`
  trait + vocabulary (`Role`, `MintContext`, `MintedSecret`,
  `SecretRef`, `EngineError`).
- `coding_adventures_vault_engine_kv2` — the first concrete
  engine: versioned static KV.

Future siblings (each its own crate, all implementing the same
trait): `vault-engine-database`, `vault-engine-pki`,
`vault-engine-aws`, `vault-engine-gcp`, `vault-engine-azure`,
`vault-engine-ssh`, `vault-engine-transit`, `vault-engine-totp`,
`vault-engine-kubernetes`.

## Why split the trait into its own crate

Concrete engines pull in heavy, distinct dependency trees
(database client SDKs for Database; AWS SDK for AWS; an X.509
codec for PKI). Putting the trait in a dependency-light crate
means consumers (transports VLT11, policy VLT06, audit VLT09)
import only the trait, not the union of every engine's deps.

A workspace can compile against a *subset* of engines (e.g. an
embedded password manager that only needs KV-v2 + TOTP) without
dragging in the AWS SDK.

## SecretEngine trait

```rust
pub trait SecretEngine: Send + Sync {
    fn mount_path(&self) -> &str;
    fn mint(&self, role: &Role, ctx: &MintContext)
        -> Result<MintedSecret, EngineError>;
    fn revoke(&self, secret_ref: &SecretRef)
        -> Result<(), EngineError>;
    fn rotate_root(&self) -> Result<(), EngineError>;
}
```

Vocabulary:

```rust
pub struct Role { pub name: String, pub default_ttl_ms: Option<u64>, pub max_ttl_ms: Option<u64> }
pub struct MintContext {
    pub principal: String,
    pub now_ms: u64,
    pub requested_ttl_ms: u64,
    // engine-specific input — engines read what they need:
    pub path: Option<String>,
    pub input: Option<Zeroizing<Vec<u8>>>,
    pub cas_token: Option<u64>,
}
pub struct MintedSecret { pub body: Zeroizing<Vec<u8>>, pub secret_ref: SecretRef, pub granted_ttl_ms: u64 }
pub enum SecretRef { KvV2 { path, version }, DbUsername(String), PkiSerial(Vec<u8>), AwsSession(String), Other(String) }  // #[non_exhaustive]
pub enum EngineError { UnknownRole(String), InvalidParameter(&'static str), Backend(String), Crypto(String), PrincipalDenied(String), UnknownSecret, Conflict }
```

Per-engine input rides on `MintContext`. The engine reads
whichever fields it needs; callers leave the rest `None`. This
keeps each `mint` call self-contained — there is no shared
"staged write" slot a concurrent caller could clobber (a
confused-deputy hazard avoided by construction).

`MintContext` deliberately does *not* derive `Clone` or `Debug`:
`input` carries plaintext under `Zeroizing`, so cloning would
duplicate plaintext into a non-zeroizing intermediate, and
`Debug` would let `dbg!` leak the bytes.

`MintedSecret::body` is held under `Zeroizing<Vec<u8>>` and has
a redacted `Debug` impl — `dbg!(minted)` cannot leak bytes.
`MintedSecret::into_lease_payload(self) -> LeasePayload` is the
canonical bridge between VLT08 and VLT07.

## How VLT08 sits between VLT07 and VLT11

```text
   transports (CLI / HTTP / gRPC, VLT11)
          │ dispatch on mount_path
   ┌──────▼──────┐
   │ Box<dyn     │   ◄── this trait
   │ SecretEngine│
   │   ├─ KvV2   │
   │   ├─ DB (future)
   │   ├─ PKI (future)
   │   ├─ AWS (future)
   │   └─ Transit (future)
   └──────┬──────┘
          │ MintedSecret { body, secret_ref, granted_ttl_ms }
   ┌──────▼──────┐
   │ LeaseManager (VLT07)
   │ wraps body in TTL'd lease, returns LeaseId
   └─────────────┘
```

## KV-v2 engine — semantics

KV-v2 is "dynamic" in the trait sense (mint/revoke/rotate API)
even though its data is static (the bytes you write are exactly
the bytes you read back).

### Mint protocol

`mint(role, ctx)` reads `ctx.path`, `ctx.input`, and
`ctx.cas_token` directly. Each call is fully self-contained —
no shared staging slot.

```rust
let ctx = MintContext {
    principal: "alice".into(),
    now_ms: 0,
    requested_ttl_ms: 60_000,
    path: Some("shared/db-password".into()),
    input: Some(Zeroizing::new(b"hunter2".to_vec())),
    cas_token: None,
};
let minted = engine.mint(&Role::new("shared"), &ctx)?;
```

### Versioning

Every successful `mint` allocates a new monotonic integer
version at the path. CAS via `ctx.cas_token`:

| `cas_token`        | Semantics                                          |
|--------------------|----------------------------------------------------|
| `None`             | unconditional write                                |
| `Some(0)`          | create-only — fails if any live version exists     |
| `Some(N > 0)`      | update — current latest-live must equal `N`        |
| `Some(n > u32::MAX as u64)` | rejected as `InvalidParameter`             |

A failed CAS returns `EngineError::Conflict`.

### Revoke

`revoke(SecretRef::KvV2 { path, version })` is a **soft delete**:
the version is marked `destroyed` and its `Zeroizing<Vec<u8>>`
body is dropped (and scrubbed). Reads of a destroyed version
return `EngineError::UnknownSecret`. The tombstone row stays in
the table indefinitely so the audit log can answer "this version
existed and was destroyed at time T" without a separate tombstone
schema. Tombstones are not subject to the `max_versions` cap.

Idempotent: re-revoking a destroyed version is `Ok(())`.

A `SecretRef` of a non-KV variant returns `UnknownSecret` — the
engine simply does not own that ref.

### Rotate root

Bumps an engine-level generation counter. In a storage-backed
sibling crate this would re-wrap every row under a new DEK; in
the in-memory reference implementation it exists as a hook so
audit-log consumers see the rotation event.

### TTL clamping

Effective granted TTL = `min(requested_or_default,
role.max_ttl_ms, engine.max_ttl_ms)`.

A zero requested TTL falls back to `role.default_ttl_ms` →
`engine.default_ttl_ms`. A clamped-to-zero TTL is rejected as
`EngineError::InvalidParameter`.

### `max_versions` cap

`KvV2Config.max_versions` (default 16) caps the number of *live*
versions each path retains. Once the live count exceeds the cap,
the oldest live row is evicted and its `Zeroizing<Vec<u8>>` body
is scrubbed. Destroyed tombstones are kept indefinitely (their
bodies are already empty `Zeroizing`s) to preserve the audit
trail.

## Threat model & test coverage (KV-v2)

| Threat                                                       | Defence                                                    | Test                                                         |
|--------------------------------------------------------------|------------------------------------------------------------|--------------------------------------------------------------|
| Plaintext leak via `dbg!(MintedSecret)`                      | hand-rolled redacted `Debug` on `MintedSecret`             | `minted_secret_debug_redacts_body` (engine-core)             |
| Plaintext leak via `dbg!(MintContext)` or `ctx.clone()`      | `MintContext` does not derive `Clone` or `Debug`           | structural                                                   |
| **Cross-caller plaintext mix-up via shared staging slot**    | per-call inputs ride on `MintContext`; no shared staged    | `concurrent_mints_do_not_cross_streams` (16 threads)         |
| Bytes linger after revocation                                | `revoke` replaces body with empty `Zeroizing` (scrubs)     | structural — `Zeroizing` Drop                                |
| Bytes linger after `max_versions` eviction                   | live evicted row's `Zeroizing<Vec<u8>>` drops + scrubs     | `max_versions_caps_live_history_and_keeps_tombstones`        |
| Tombstones evicted (audit-log integrity)                     | eviction counts only live rows; tombstones kept            | `max_versions_caps_live_history_and_keeps_tombstones`        |
| Non-zeroizing `Vec<u8>` intermediates during clone           | bodies wrapped in `Zeroizing` immediately on creation      | structural                                                   |
| Concurrent writers race on same path                         | CAS via `ctx.cas_token`; loser → `Conflict`                | `cas_create_only_rejects_overwrite`, `cas_update_from_wrong_version_rejected`, `cas_update_from_correct_version_accepted` |
| Caller passes `cas_token > u32::MAX`                         | `mint` rejects with `InvalidParameter`                     | `cas_token_overflow_u32_rejected`                            |
| Caller forgets path / input                                  | `mint` errors `InvalidParameter`                           | `mint_without_path_errors`, `mint_without_input_errors`      |
| Caller passes empty path / input                             | `mint` errors `InvalidParameter`                           | `mint_with_empty_path_errors`, `mint_with_empty_input_errors`|
| Re-revoke creates spurious error                             | `revoke` is idempotent on destroyed versions               | `revoke_idempotent`                                          |
| Wrong-variant `SecretRef` fed to `revoke`                    | returns `UnknownSecret` (engine doesn't own that ref)      | `revoke_wrong_variant_returns_unknown_secret`                |
| Caller-requested TTL exceeds engine policy                   | TTL clamping via `clamp_ttl`                               | `ttl_clamped_to_engine_max`, `ttl_clamped_to_role_max`       |
| Caller-supplied TTL = 0 silently bypasses TTL                | falls back to role default → engine default; rejects 0     | `zero_request_uses_default_ttl`                              |
| Empty mount path collides with reserved value                | `KvV2Engine::new` returns `InvalidParameter` (no panic)    | `empty_mount_path_returns_invalid_parameter`                 |
| Version counter overflow                                     | `checked_add` on next-version; `InvalidParameter` on overflow | structural                                                |
| Read of destroyed version returns stale bytes                | destroyed flag is checked before returning bytes           | `revoke_makes_version_unreadable`                            |
| Read of non-existent path leaks "exists?" oracle             | uniform `UnknownSecret` for missing path and missing version | `read_unknown_path_is_unknown_secret`                      |

## Out of scope (future PRs)

- **Encryption at rest** — a storage-backed sibling crate will
  route rows through VLT01 sealed-store. The reference
  implementation here is in-memory.
- **Patch operations** — HashiCorp's KV-v2 has a JSON-merge
  patch op; we keep the surface to whole-row writes.
- **Recursive `list`** — listing children of a path. Easy
  follow-up; not in the trait surface yet.
- **Cross-process locking** — a storage-backed sibling will use
  the storage backend's CAS primitives.
- **Other engines** — Database, PKI, AWS, GCP, Azure, SSH,
  Transit, TOTP, Kubernetes. Each is a sibling crate; the trait
  is now stable for them to land against.

## Citations

- HashiCorp Vault — *Secret Engines* design (mint / revoke /
  rotate vocabulary).
- HashiCorp KV v2 — versioned KV semantics, CAS via
  `expected_version`, soft-delete + destroy distinction.
- VLT00-vault-roadmap.md — engine tier placement.
- VLT07-vault-leases.md — what wraps the minted bytes.
- VLT01-vault-sealed-store.md — what a storage-backed engine
  will route through.
- `coding_adventures_zeroize::Zeroizing` — body scrubbing.
