# `coding_adventures_vault_engine_core` — VLT08 trait

The `SecretEngine` trait + vocabulary types every Vault dynamic-
secret engine implements. Concrete engines (KV-v2, Database, PKI,
AWS, GCP, Azure, SSH, Transit, TOTP, Kubernetes) live in their own
sibling crates and all satisfy this one trait.

Keeping the trait in a dependency-light crate means a downstream
build can pull in only the engines it needs (e.g. an embedded
password manager: `vault-engine-kv2` + `vault-engine-totp` and
nothing else) without dragging in DB drivers / cloud SDKs.

## API at a glance

```rust
pub trait SecretEngine: Send + Sync {
    fn mount_path(&self) -> &str;
    fn mint(&self, role: &Role, ctx: &MintContext) -> Result<MintedSecret, EngineError>;
    fn revoke(&self, secret_ref: &SecretRef) -> Result<(), EngineError>;
    fn rotate_root(&self) -> Result<(), EngineError>;
}
```

Vocabulary:

| Type            | What it carries                                              |
|-----------------|--------------------------------------------------------------|
| `Role`          | role name + optional default/max TTL                         |
| `MintContext`   | principal + caller-supplied `now_ms` + requested TTL         |
| `MintContextSummary` | value-redacted shape summary for planning/audit logs    |
| `MintedSecret`  | zeroizing body + `SecretRef` + granted TTL                   |
| `SecretRef`     | non-exhaustive enum: KvV2 / DbUsername / PkiSerial / AwsSession / Other |
| `EngineError`   | `UnknownRole` / `InvalidParameter` / `Backend` / `Crypto` / `PrincipalDenied` / `UnknownSecret` / `Conflict` |

## How it slots into the stack

```text
   transports (CLI/HTTP/gRPC, VLT11)
         │ dispatch on mount_path
   ┌─────▼─────┐
   │  dyn      │   ◄── this trait
   │ Secret-   │       (KV-v2, DB, PKI, AWS, …)
   │ Engine    │
   └─────┬─────┘
         │ MintedSecret
   ┌─────▼─────┐
   │ Lease-    │   wraps minted bytes in TTL'd lease
   │ Manager   │   (VLT07)
   └───────────┘
```

The flow is "engine mints → lease wraps → audit records → caller
receives a `LeaseId`". Every engine speaks the same shape so the
top-level orchestration is engine-agnostic.

## Threat model

- **`MintedSecret::body`** is held under `Zeroizing<Vec<u8>>` and
  has a redacted `Debug` impl — `dbg!(minted)` cannot leak the
  bytes.
- **`MintContext.now_ms`** is caller-supplied so engines stay
  pure (no syscall on the hot path) and tests are deterministic.
- **`MintContext.principal`** is an audit breadcrumb; engines
  *may* additionally `defence-in-depth` check it (e.g. AWS
  engine enforcing a per-principal allow-list inside `mint`),
  surfacing `EngineError::PrincipalDenied`.
- **`MintContext::summary()`** reports only shape and byte counts,
  not principal/path values, CAS tokens, or plaintext input bytes.
- **`SecretRef::Other`** lets a future engine return a handle
  without the trait crate growing every release.

## Capabilities

None — pure trait + vocabulary. See `required_capabilities.json`.

See [`VLT00-vault-roadmap.md`](../../../specs/VLT00-vault-roadmap.md)
and [`VLT08-vault-dynamic-secrets.md`](../../../specs/VLT08-vault-dynamic-secrets.md).
