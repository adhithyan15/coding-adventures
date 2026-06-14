# VLT09 — Vault Audit Log

## Overview

Append-only, tamper-evident, hash-chained, Ed25519-signed audit
log. Every operation that mutates the vault — `mint`, `revoke`,
`rotate_root`, `auth_*`, `policy_*`, `lease_*`, `sealed_*` —
produces one signed entry, and every entry binds to the entry
before it via `prev_hash = blake2b-256(prev.canonical ||
this.body)`.

Implementation lives at `code/packages/rust/vault-audit/`.

## Why this layer exists

VLT07/VLT08 makes security-critical decisions; VLT05/VLT06
authenticates and authorises them; VLT01 stores the result.
Without VLT09, a compromise is undetectable — an attacker mints
a credential and walks away. With this layer, every mint is a
signed entry in an append-only chain, visible to every honest
party with the device's public key, even when the storage server
is malicious.

It is also the substrate for compliance-driven workflows (SOC 2,
ISO 27001, HIPAA, FedRAMP) that require a non-repudiable trail
of who-did-what-when.

## Two stacked primitives

1. **Hash chain (integrity over the past)**
   Each entry's `prev_hash` is
   `blake2b-256(prev_entry.canonical_bytes() ||
   this_entry.body_bytes())`. A storage layer that mutates any
   entry breaks every later `prev_hash`; verification fails on
   the next pass. The genesis entry's `prev_hash` is 32 zero
   bytes.

2. **Ed25519 signature (authenticity)**
   Every entry is signed with the issuer's long-term device
   key. An attacker without that key cannot forge an entry —
   they can drop the chain or refuse to deliver it (DoS) but
   cannot lie about what happened.

Together: the storage layer can be untrusted (multi-tenant
cloud, sync server) and the chain still detects tampering.
This is the same threat model as Sigstore Rekor, Trillian,
Sigsum, and HashiCorp Vault's audit device.

## Public API

```rust
pub enum AuditAction {           // #[non_exhaustive]
    AuthSucceed, AuthFail,
    PolicyAllow, PolicyDeny,
    EngineMint, EngineRevoke, EngineRotateRoot,
    LeaseConsume, LeaseRevoke,
    SealedWrite, SealedRead,
    Other(String),
}

pub struct AuditEvent {
    pub principal: String,
    pub action: AuditAction,
    pub resource: Option<String>,
    pub detail: Option<Vec<u8>>,
}

pub struct AuditEntry {
    pub seq: u64,
    pub timestamp_ms: u64,
    pub prev_hash: [u8; 32],
    pub event: AuditEvent,
}

pub struct SignedAuditEntry {
    pub entry: AuditEntry,
    pub signer_pub: [u8; 32],
    pub signature: [u8; 64],
}

pub trait AuditSink: Send + Sync {
    fn append(&self, entry: SignedAuditEntry) -> Result<(), AuditError>;
    fn len(&self)     -> Result<u64, AuditError>;
    fn entries(&self) -> Result<Vec<SignedAuditEntry>, AuditError>;
    fn last(&self)    -> Result<Option<SignedAuditEntry>, AuditError>;
}

pub struct InMemoryAuditSink;
pub struct AuditSigningKey;        // wraps Zeroizing<[u8; 64]> + redacted Debug
pub struct AuditChain<S: AuditSink>;

pub fn verify_chain(
    entries: &[SignedAuditEntry],
    expected_signer_pub: Option<&[u8; 32]>,
) -> Result<(), AuditError>;
```

`AuditChain::attach(key, sink)` runs `verify_chain` end-to-end
against the existing sink contents (pinned to `key.public()`)
*before* enabling new writes, so a process restart over a
tampered sink fails closed instead of silently extending a
corrupt chain. `AuditChain::attach_unverified` is the escape
hatch for callers with a separate proof of integrity.

## Canonical bytes

The signed bytes need to be reproducible byte-for-byte. We use a
tagged length-prefixed framing rather than CBOR/JSON because:

- zero deps inside this crate;
- trivially reviewable (one function, < 80 lines);
- inputs are bounded so the encoder cannot blow up.

```text
"AUD1"                 4 bytes magic
u8  version            = 1
u64 seq                BE
u64 timestamp_ms       BE
[u8;32] prev_hash
u8  action_tag
u8  has_other_label    (1 if AuditAction::Other, else 0)
u32 other_label_len    || other_label_bytes  (0/empty otherwise)
u32 principal_len      || principal_bytes
u8  has_resource       || u32 len || bytes (or 0/empty)
u8  has_detail         || u32 len || bytes (or 0/empty)
```

`event_body_bytes(ev)` is the same encoding minus the framing
fields — used as the second argument to the chain hash.

## Bounds (tested)

- `principal` non-empty, ≤ 256 chars
- `resource` ≤ 512 chars
- `detail` ≤ 1024 bytes
- `Other(label)` non-empty, ≤ 64 chars

These caps keep verification linear in entries (not in bytes of
detail), and prevent a malicious caller from amplifying log
size.

## Threat model & test coverage

| Threat                                                       | Defence                                                | Test                                                   |
|--------------------------------------------------------------|--------------------------------------------------------|--------------------------------------------------------|
| Storage tampers with an entry's body                         | hash chain — every later `prev_hash` breaks            | `tampered_event_breaks_chain`                          |
| Storage tampers with an *earlier* entry only                 | next entry's `prev_hash` was bound to the old bytes    | `tampered_first_entry_breaks_second_via_chain`         |
| Storage truncates the head                                   | sequence number must start at 0                        | `truncation_visible_as_resequence`                     |
| Storage forges a fresh entry                                 | Ed25519 signature requires device's secret key         | `forged_signature_breaks_verification`                 |
| Adversary swaps signing key + re-signs all entries           | `verify_chain(.., Some(pinned_pub))`                   | `pinned_issuer_check`                                  |
| Reorder entries                                              | seq numbers + prev_hash linkage                        | `truncation_visible_as_resequence` (covers reorder via missing seq) |
| `dbg!(signing_key)` leaks secret bytes                       | hand-rolled redacted `Debug` on `AuditSigningKey`      | `signing_key_debug_redacts_secret`                     |
| Caller-supplied oversized fields blow up sink                | `validate_event` rejects up front                      | `validate_rejects_oversize_principal`, `validate_rejects_oversize_detail`, `validate_rejects_oversize_other_label` |
| Empty / unrecognised principal                               | `validate_event` rejects                               | `validate_rejects_empty_principal`                     |
| Empty `Other` label                                          | `validate_event` rejects                               | `validate_rejects_empty_other_label`                   |
| Sequence number overflow on a 64-bit chain                   | `checked_add` returns `SinkError`                      | structural                                             |
| Process restart loses chain head                             | `attach()` reads sink tail to recover head             | `attach_picks_up_existing_chain`                       |
| **Restart attaches to a tampered sink tail**                 | `attach()` verifies entire chain end-to-end against the supplied signing key; fails closed with `VerificationFailed` | `attach_rejects_tampered_chain` |
| Restart attaches with a swapped signing key                  | `attach()` pins to the key it was given                | `attach_rejects_mismatched_signer`                     |
| Mutex poisoning silently DoSes the audit log                 | `lock_recover` recovers via `PoisonError::into_inner`  | structural — invariants verified coherent              |
| Concurrent `record` calls race                               | head cached under `Mutex`; serialise                   | `concurrent_record_serializes_via_mutex` (32 threads)  |
| Sink failure mid-record advances head spuriously             | append-then-bump-head order; head untouched on error   | structural                                             |
| Genesis entry has unknown predecessor                        | `prev_hash = [0; 32]` for `seq == 0`                   | `genesis_has_zero_prev_hash`                           |

## Out of scope (future PRs)

- Persistent file sink — `vault-audit-fs`.
- Transparency log — `vault-audit-trillian` / `vault-audit-sigsum`.
- Cloud sinks — `vault-audit-syslog`, `vault-audit-s3`,
  `vault-audit-splunk`.
- Sealed-at-rest — production deployments wrap each
  `SignedAuditEntry::canonical_bytes()` in VLT01 sealed-store
  before persistence; the chain still verifies because hashes
  are over the cleartext bytes computed before sealing.
- Indexed queries — search by principal / action / time range
  over the chain.

## Citations

- HashiCorp Vault — Audit device design (hash-chained log,
  Ed25519 signatures over canonical entries).
- Sigstore Rekor / Trillian / Sigsum — transparency log
  integrity model.
- VLT00-vault-roadmap.md — VLT09 placement.
- VLT01-vault-sealed-store.md — what wraps each canonical entry
  for at-rest confidentiality.
- VLT05-vault-auth.md — source of the device signing key
  identity.
- `coding_adventures_blake2b::blake2b` — chain hash primitive.
- `coding_adventures_ed25519::sign`/`verify` — signature primitive.
- `coding_adventures_zeroize::Zeroizing` — secret-key wiping.
