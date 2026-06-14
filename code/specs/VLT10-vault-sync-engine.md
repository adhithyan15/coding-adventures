# VLT10 — Vault Sync Engine

## Overview

The multi-device sync layer between VLT01 sealed-store and the
VLT11 transports. It owns:

- **Per-record version vectors** for causality tracking.
- **Last-writer-wins resolver** for concurrent edits, with
  conflicts *surfaced* (not silently dropped) so the application
  layer can prompt the user.
- **Opt-in OR-set CRDT** for fields where LWW is wrong (most
  notoriously a tag list).
- **`SyncServer` trait + `InMemorySyncServer` reference**.
  Persistent / cloud variants land as sibling crates.

Implementation lives at `code/packages/rust/vault-sync/`.

## Storage-agnostic by construction

The whole point of putting a sync layer between
`vault-sealed-store` (VLT01) and an untrusted server is that
the server learns *nothing* about contents:

```text
SyncRecord {
    namespace: "vault/login",       // server-visible
    key: "github",                  // server-visible
    version_vector: { A: 3, B: 2 }, // server-visible
    last_writer: DeviceId(...),     // server-visible
    last_writer_ms: u64,            // server-visible
    ciphertext: [u8],               // OPAQUE
    wrap_set: Option<[u8]>,         // OPAQUE (VLT04)
}
```

`ciphertext` is `Vec<u8>` and the server never decrypts. The
wrap-set (recipient list keyed by public key) is similarly
opaque from the server's perspective; clients fetch both halves
together so any authorised reader can decrypt.

## Public API

```rust
pub struct DeviceId(/* opaque, bounded string */);

pub enum VectorOrdering { Equal, Dominates, DominatedBy, Concurrent }

pub struct VersionVector {
    counters: BTreeMap<DeviceId, u64>,
}
impl VersionVector {
    pub fn new()                                     -> Self;
    pub fn bump(self, device: &DeviceId)             -> Self;
    pub fn get(&self, device: &DeviceId)             -> u64;
    pub fn merge(&self, other: &Self)                -> Self;
    pub fn compare(&self, other: &Self)              -> VectorOrdering;
    pub fn dominates(&self, other: &Self)            -> bool;
    pub fn concurrent_with(&self, other: &Self)      -> bool;
}

pub struct SyncRecord {
    pub namespace: String,
    pub key: String,
    pub version_vector: VersionVector,
    pub last_writer: DeviceId,
    pub last_writer_ms: u64,
    pub ciphertext: Vec<u8>,
    pub wrap_set: Option<Vec<u8>>,
}

pub enum PushOutcome {
    Applied { stored: SyncRecord },
    Stale { server: SyncRecord },
    ConflictResolved { winner: SyncRecord, loser: SyncRecord },
    Unchanged,
}

pub trait SyncServer: Send + Sync {
    fn push(&self, record: SyncRecord)  -> Result<PushOutcome,    SyncError>;
    fn get(&self, namespace: &str, key: &str) -> Result<SyncRecord, SyncError>;
    fn pull(&self, namespace: &str, since: &VersionVector)
        -> Result<Vec<SyncRecord>, SyncError>;
}

pub struct InMemorySyncServer;
pub struct LwwResolver;
pub struct OrSet;  // observed-removal Set CRDT
```

## Conflict semantics

| Vector relationship                    | Outcome                                       |
|----------------------------------------|-----------------------------------------------|
| incoming dominates server              | `Applied`                                     |
| incoming equal + same bytes/writer     | `Unchanged`                                   |
| incoming equal + bytes differ          | `ConflictResolved` — protocol violation by clients (they didn't bump their counter); server defends with LWW |
| incoming dominated by server           | `Stale { server }` — client should pull-and-rebase |
| concurrent (incomparable vectors)      | `ConflictResolved { winner, loser }`          |

LWW tie-break: higher `last_writer_ms` wins; on equal
timestamps, lexicographically *smaller* `last_writer` `DeviceId`
wins (deterministic across replicas — same answer no matter
which server in a multi-leader topology evaluates first).

## OR-set CRDT

For fields where LWW is unsuitable. A tag list:

- Device A adds "work".
- Device B adds "personal".

Under LWW the loser's tag is silently dropped. Under OR-set
merge, both adds survive:

```rust
let mut a = OrSet::new();
a.add("work", &device_a, now_ms);
let mut b = OrSet::new();
b.add("personal", &device_b, now_ms);
let merged = a.merge(&b);
// merged.contains("work") && merged.contains("personal")
```

Implementation uses observed-removal semantics: add and remove
track per-(device, value) tags, so add → remove → re-add across
devices behaves correctly.

`merge` is idempotent, commutative, and associative
(CRDT invariant — verified by tests).

## Bounds

| Field            | Cap                |
|------------------|--------------------|
| `DeviceId`       | 256 bytes          |
| `namespace`      | 128 bytes          |
| `key`            | 512 bytes          |
| `ciphertext`     | 1 MiB per record   |
| `wrap_set`       | 64 KiB per record  |

Capping `ciphertext` to 1 MiB means large attachments need to
be chunked at a higher tier (VLT14 attachments). This keeps
sync replication cheap and bounds memory use per push.

## Threat model & test coverage

| Threat                                                       | Defence                                                | Test                                                   |
|--------------------------------------------------------------|--------------------------------------------------------|--------------------------------------------------------|
| Server reads plaintext                                       | server only sees `Vec<u8>`; encryption is VLT01's job  | structural — `ciphertext` is opaque                    |
| Server identifies record types                               | server never sees record framing; just bytes           | structural                                             |
| Replay of stale write                                        | `Stale` outcome on dominated incoming vector           | `dominated_push_returns_stale`                         |
| Idempotent retry double-applies                              | equal vector + equal bytes → `Unchanged`               | `idempotent_push_returns_unchanged`                    |
| Two writers collide under same vector (protocol violation)   | server defends with LWW (`ConflictResolved`)           | `equal_vector_with_different_bytes_is_treated_as_conflict` |
| Concurrent legitimate edits silently lose data               | `ConflictResolved` returns both records                | `concurrent_push_resolves_via_lww`                     |
| LWW tie-break flaps across servers                           | smaller-DeviceId wins deterministically                | `lww_breaks_ties_by_smaller_device_id`                 |
| Tag-list collapse under LWW                                  | OR-set merge unions concurrent adds                    | `orset_merge_unions_concurrent_adds`                   |
| OR-set merge order matters                                   | merge is idempotent + commutative                      | `orset_merge_idempotent_commutative`                   |
| OR-set add-remove-readd loses information                    | observed-removal semantics                             | `orset_readd_after_remove_works`                       |
| Memory amplification via huge fields                         | bounded `MAX_*_LEN` enforced on every push             | `validate_rejects_oversize_ciphertext`, `device_id_rejects_oversize` |
| Caller forgets last_writer in vector                         | `validate` rejects `version_vector[last_writer] == 0`  | `validate_rejects_vector_without_last_writer`          |
| Empty namespace / key / ciphertext                           | `validate` rejects                                     | `validate_rejects_empty_namespace`, `validate_rejects_empty_key`, `validate_rejects_empty_ciphertext` |
| Cross-namespace leak in `pull`                               | `pull` filters by namespace                            | `pull_skips_other_namespaces`                          |
| Concurrent pushes corrupt server state                       | `Mutex<HashMap>`; per-record CAS via `compare`         | `concurrent_pushes_all_resolve` (16 threads)           |
| **Wrap-set rotation push silently dropped under equal vector** | equality now requires ciphertext + last_writer + last_writer_ms + wrap_set all match | `equal_vector_same_bytes_different_wrap_set_is_conflict` |
| **OrSet tag collision across independent instances**         | `add_with_tag(value, device, tag_id, now_ms)` for caller-supplied globally unique tags | `or_set_add_with_tag_supports_independent_instances` |
| Log-injection via control chars in `DeviceId`/namespace/key  | `is_safe_id_string` rejects control + whitespace       | `device_id_rejects_control_chars`, `validate_rejects_namespace_with_control_chars`, `validate_rejects_key_with_control_chars` |
| `Debug` on `SyncRecord` leaks ciphertext + wrap_set bytes    | hand-rolled redacted `Debug` (lengths only)            | `debug_redacts_ciphertext_and_wrap_set`                |
| Mutex poisoning silently DoSes the server                    | `lock_recover` recovers via `PoisonError::into_inner`  | structural — invariants verified coherent              |

## Out of scope (future PRs)

- Persistent / cloud SyncServer impls (Postgres, SQLite, S3).
- Authenticated `push`/`pull` (delegated to VLT05/VLT06).
- Wire transport (delegated to VLT11).
- Attachment chunking (VLT14).
- Tombstone GC for OR-set after every device has observed.
- More CRDT types (LWW-Register, PN-Counter).

## Citations

- VLT00-vault-roadmap.md — VLT10 placement.
- Lamport, "Time, Clocks, and the Ordering of Events" — version
  vector semantics.
- Shapiro et al., "A comprehensive study of Convergent and
  Commutative Replicated Data Types" — observed-removal Set.
- Bitwarden / 1Password sync designs — LWW-by-default with
  conflict surfacing.
- VLT01-vault-sealed-store.md — what produces the ciphertext.
- VLT04-vault-recipients.md — what produces the wrap-set.
- VLT11-transports — what carries this protocol over the wire.
