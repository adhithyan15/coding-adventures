# `coding_adventures_vault_audit` — VLT09 audit log

Append-only, tamper-evident, hash-chained, Ed25519-signed audit
log for the Vault stack. Each entry binds itself to the
previous via `prev_hash = blake2b-256(prev.canonical || this.body)`,
and the whole entry is signed with the issuer's device key.

## Quick example

```rust
use coding_adventures_vault_audit::{
    AuditAction, AuditChain, AuditEvent, AuditSigningKey,
    InMemoryAuditSink, verify_chain,
};

let key = AuditSigningKey::from_seed(&[0x42; 32]); // pull seed from CSPRNG / VLT03 in real use
let chain = AuditChain::attach(key, InMemoryAuditSink::new())?;

chain.record(
    AuditEvent {
        principal: "alice".into(),
        action: AuditAction::EngineMint,
        resource: Some("kv/shared".into()),
        detail: None,
    },
    1_700_000_000_000,
)?;

// Later, on a verifier with the device's public key:
let entries = chain.sink().entries()?;
verify_chain(&entries, Some(&chain.signer_public()))?;
```

## What goes in the chain

`AuditAction` is a non-exhaustive enum so new actions can land
without breaking the wire format:

| Action               | Layer  |
|----------------------|--------|
| `AuthSucceed`        | VLT05  |
| `AuthFail`           | VLT05  |
| `PolicyAllow`        | VLT06  |
| `PolicyDeny`         | VLT06  |
| `EngineMint`         | VLT08  |
| `EngineRevoke`       | VLT08  |
| `EngineRotateRoot`   | VLT08  |
| `LeaseConsume`       | VLT07  |
| `LeaseRevoke`        | VLT07  |
| `SealedWrite`        | VLT01  |
| `SealedRead`         | VLT01  |
| `Other(label)`       | escape hatch |

## Threat model

| Attack                                | Caught by                                          |
|---------------------------------------|----------------------------------------------------|
| Storage server tampers with an entry  | Hash chain — every later `prev_hash` mismatches    |
| Storage server tampers with last entry alone | Ed25519 signature on that entry             |
| Storage server drops the head         | Sequence-number-must-start-at-0 check              |
| Storage server forges a new entry     | Cannot sign without device's secret key            |
| Adversary swaps issuer (re-signs all) | `verify_chain(.., Some(pinned_pub))` rejects       |
| Reorder of entries                    | Sequence numbers + `prev_hash` linkage             |
| `dbg!(signing_key)` leaks secret      | `AuditSigningKey` `Debug` is hand-rolled redacted  |
| Caller supplies oversize detail/principal/resource/Other-label | `validate_event` rejects up front |

## What it is NOT

- Not a transparency log: no Merkle tree, no inclusion / consistency
  proofs. A future `vault-audit-trillian` sibling crate adds those.
- Not a sealing layer: VLT01 sealed-store is the canonical
  encryption-at-rest. This crate produces canonical bytes; sinks
  that need sealing wrap them in VLT01 before persisting.
- Not a query engine: `entries()` is a forward scan. Indexed
  queries belong in a higher tier.
- Not a clock: `record(event, timestamp_ms)` takes time as a
  parameter so the crate stays deterministic and capability-free.

## Where it fits

```text
   VLT07 leases / VLT08 engines / VLT06 policy / VLT05 auth
                          │ AuditEvent
   ┌──────────────────────▼─────────────────────────────────┐
   │            AuditChain  (THIS CRATE)                    │
   │   - allocate next seq                                   │
   │   - link prev_hash via blake2b-256                      │
   │   - sign with Ed25519 device key                        │
   └──────────────────────┬─────────────────────────────────┘
                          │ SignedAuditEntry
   ┌──────────────────────▼─────────────────────────────────┐
   │            AuditSink                                    │
   │     ├─ InMemoryAuditSink (this crate)                   │
   │     ├─ vault-audit-fs (future)                          │
   │     ├─ vault-audit-trillian (future)                    │
   │     └─ vault-audit-syslog (future)                      │
   └─────────────────────────────────────────────────────────┘
```

## Capabilities

None — pure crypto. See `required_capabilities.json`.

See [`VLT00-vault-roadmap.md`](../../../specs/VLT00-vault-roadmap.md)
and [`VLT09-vault-audit-log.md`](../../../specs/VLT09-vault-audit-log.md).
