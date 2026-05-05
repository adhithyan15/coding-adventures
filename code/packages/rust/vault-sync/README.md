# `coding_adventures_vault_sync` — VLT10 sync engine

Multi-device E2EE sync layer for the Vault stack. Per-record
version vectors with last-writer-wins by default + opt-in CRDT
(OR-set) for fields that need true union semantics. The server
sees only ciphertext + ordering metadata — never plaintext or
record type.

## Quick example

```rust
use coding_adventures_vault_sync::{
    DeviceId, InMemorySyncServer, PushOutcome, SyncRecord,
    SyncServer, VersionVector,
};

let server = InMemorySyncServer::new();
let device_a = DeviceId::new("alice-laptop")?;

// Encrypt your record above this layer (VLT01 sealed-store).
let ciphertext: Vec<u8> = encrypt_with_vlt01(plaintext);

let record = SyncRecord {
    namespace: "vault/login".into(),
    key: "github".into(),
    version_vector: VersionVector::new().bump(&device_a),
    last_writer: device_a.clone(),
    last_writer_ms: 1_700_000_000_000,
    ciphertext,
    wrap_set: Some(wrap_set_bytes),
};

match server.push(record)? {
    PushOutcome::Applied { stored } => { /* normal write */ }
    PushOutcome::Stale { server }   => { /* pull-and-rebase */ }
    PushOutcome::ConflictResolved { winner, loser } => {
        /* surface conflict UI */
    }
    PushOutcome::Unchanged => { /* idempotent retry */ }
}
```

## Wire shape

```text
SyncRecord {
    namespace: "vault/login",       // server-visible
    key: "github",                  // server-visible
    version_vector: { A: 3, B: 2 }, // server-visible
    last_writer: DeviceId(...),     // server-visible
    last_writer_ms: u64,            // server-visible
    ciphertext: [u8],               // OPAQUE — never decrypted by server
    wrap_set: Option<[u8]>,         // OPAQUE — VLT04 recipient list
}
```

The server **never** sees plaintext, never knows the record's
type, and cannot tell what's encoded inside the ciphertext.

## Conflict policy

| Vector relationship                  | Outcome             |
|--------------------------------------|---------------------|
| incoming dominates server            | `Applied`           |
| incoming equal + bytes match         | `Unchanged`         |
| incoming equal + bytes differ        | `ConflictResolved` (LWW) |
| incoming dominated by server         | `Stale`             |
| concurrent (incomparable vectors)    | `ConflictResolved` (LWW) |

LWW tie-break: higher `last_writer_ms` wins; if tied, the
lexicographically *smaller* `last_writer` `DeviceId` wins
(deterministic across replicas).

## OR-set CRDT for fields that need real merge

LWW silently loses concurrent additions to a list — most
notoriously a tag list ("work" added on device A, "personal"
added on device B → LWW keeps one). For those fields, wrap in
`OrSet`:

```rust
use coding_adventures_vault_sync::{DeviceId, OrSet};

let mut a = OrSet::new();
a.add("work", &device_a, 1);
let mut b = OrSet::new();
b.add("personal", &device_b, 1);
let merged = a.merge(&b);
assert!(merged.contains("work"));
assert!(merged.contains("personal"));
```

Add and remove are tracked per-tag so add-then-remove-then-add
across devices works correctly (observed-removal semantics).

## Threat model

- **Server is untrusted.** Sees ciphertext + version vector +
  device IDs. Cannot read, cannot infer record type.
- **Forgery.** Prevented by the layer below (VLT01 sealed-store
  + VLT04 wrap-set signatures); this layer's only forgery
  surface is "the server lies about a vector". Detected by the
  client — pulled record's vector must `>=` the local view.
- **Replay.** Stale push with old vector → `PushOutcome::Stale`
  rather than silent overwrite.
- **Concurrent legitimate edits.** Surfaced as
  `ConflictResolved` with both records preserved so the
  application can show a merge UI.
- **Bounded memory.** Every variable-size field is capped
  (`MAX_NAMESPACE_LEN`, `MAX_KEY_LEN`, `MAX_DEVICE_ID_LEN`,
  `MAX_CIPHERTEXT_LEN`, `MAX_WRAP_SET_LEN`); validation runs on
  every `push`.

## What this crate is NOT

- Not a transport (TLS, gRPC). VLT11's job.
- Not authentication / authorisation. VLT05/VLT06 above it.
- Not encryption. VLT01 below it. The server sees only opaque
  bytes by construction.
- Not a chunked-attachment store. VLT14.

## Capabilities

None — pure data structures. See `required_capabilities.json`.

See [`VLT00-vault-roadmap.md`](../../../specs/VLT00-vault-roadmap.md)
and [`VLT10-vault-sync-engine.md`](../../../specs/VLT10-vault-sync-engine.md).
