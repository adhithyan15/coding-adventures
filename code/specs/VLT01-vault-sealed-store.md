# VLT01 — Vault Sealed Store

## Overview

The **sealed store** is the bottom encryption layer of the Vault stack. It
sits directly on top of [`storage-core`](./storage-sqlite.md) (which stores
opaque bytes) and turns it into an **encrypted-secrets store** whose
plaintext is only readable while the vault is *unsealed*.

This document specifies the on-disk format, the seal/unseal ceremony, the
in-memory state machine, and the API surface. It does NOT cover:

- lease-key response encryption (higher layer — VLT02, future)
- secure channel / key exchange (higher layer — VLT03, future)
- secret-type codecs like passwords, SSH keys, TOTP seeds (higher layer)
- multi-party unseal / Shamir / YubiKey unlock (VLT0?, future)

### Layering

```text
        higher vault layers
                │
                ▼
        vault-sealed-store           <── this spec
                │  (envelope-encrypted records)
                ▼
         StorageBackend              (storage-core)
                │  (opaque bytes)
                ▼
   local-folder / sqlite / memory / …
```

### Why envelope encryption?

There are two choices for "encrypt everything with a single key":

1. **Direct**: AEAD every record under the master key.
2. **Envelope**: generate a fresh per-record Data-Encryption-Key (DEK),
   encrypt the body under the DEK, then encrypt the DEK under a master
   Key-Encryption-Key (KEK). Store (ciphertext, wrapped-DEK, nonces)
   together.

Envelope wins because:

- **Rotatable KEK.** Rotating the master key is O(records) wraps of 32
  bytes each, not O(records) re-encryptions of full bodies.
- **Nonce safety.** A new 32-byte DEK per record eliminates any
  collision-under-one-key concerns. Nonces for the body AEAD can safely
  be zeroed (fresh key → fresh keystream).
- **Defence in depth.** Compromise of one DEK leaks exactly one record.
- **Streaming ready.** DEK lets future large-body records split into
  chunks without extra key ceremony.

The cost is ~80 bytes of metadata overhead per record. That is
acceptable for a secrets store; if we ever wrap large attachments this
may need revisiting.

## On-disk layout

Every secret is persisted as one `StorageRecord`:

```text
namespace = "<caller-chosen>"
key       = "<caller-chosen>"
content_type = "application/vault-sealed+json-v1"
body      = <ciphertext bytes>          (variable, may be empty)
metadata  = {                            (StorageMetadata JsonValue object)
  "vault_sealed_version": 1,
  "aead": "xchacha20poly1305",
  "body_nonce": "<base16 24 bytes>",
  "body_tag":   "<base16 16 bytes>",
  "body_aad":   "<base16 of namespace||0x00||key>",
  "wrapped_dek": "<base16 ciphertext of 32-byte DEK>",
  "wrapped_dek_nonce": "<base16 24 bytes>",
  "wrapped_dek_tag":   "<base16 16 bytes>",
  "kek_id": "<stable id of the KEK used to wrap this record>"
}
```

AAD binds the ciphertext to its storage address, so an attacker that
swaps two records' bodies cannot escape detection.

The **manifest** is a singleton record stored at a reserved address:

```text
namespace = "__vault__"
key       = "manifest"
content_type = "application/vault-manifest+json-v1"
metadata  = {
  "vault_manifest_version": 1,
  "kdf": "argon2id",
  "kdf_version": "0x13",
  "kdf_time_cost": <u32>,
  "kdf_memory_cost_kib": <u32>,
  "kdf_parallelism": <u32>,
  "kdf_tag_length": 32,
  "keks": [
    { "id": "kek-1", "status": "active"|"retired",
      "salt": "<base16 ≥8 bytes from CSPRNG>",
      "verifier_nonce": "<base16 24>",
      "verifier_tag":   "<base16 16>",
      "verifier_ct":    "<base16 16 bytes of AEAD'd zeros>" }
    , …
  ],
  "created_at_ms": <u64>
}
body = (empty)
```

Each KEK entry carries its own salt. This is the salt that was used with
the Argon2id parameters above to derive *that* KEK from the operator
password. After a rotation the old KEK entry remains in the manifest
with status `retired`, together with its original salt, so it can still
be verified (and the records it wrapped can still be unwrapped in a
crash-recovery path).

**Attacker-tampered manifest defence.** Because the manifest is sitting
on the at-rest medium, an attacker who can rewrite bytes at rest could
otherwise set `kdf_memory_cost_kib = u32::MAX` and force unseal to
allocate TiB of RAM. The implementation therefore clamps the persisted
KDF parameters to these hard ceilings before calling Argon2:

| parameter       | min                      | max           |
|-----------------|--------------------------|---------------|
| time_cost       | 1                        | 10            |
| memory_cost_kib | 8 × parallelism (RFC 9106) | 5 × 1024 × 1024 (5 GiB) |
| parallelism     | 1                        | 64            |
| salt length     | 8 bytes                  | 1024 bytes    |

A manifest outside these bounds is rejected with `Validation` before any
key-derivation work runs.

The **verifier** is a known-plaintext (16 zero bytes) AEAD'd under the
KEK. On unseal we recompute a candidate KEK from the operator password
(using the salt of each KEK entry) and check it decrypts the verifier
to zeros — if not, the password is wrong against that entry. This gives
O(entries) constant-time password checks without needing any stored
secret.

Namespace `__vault__` is reserved. The sealed-store rejects writes with
that namespace from external callers.

### Namespace registry

`storage-core` does not expose a "list all namespaces" operation. To
support `rotate_kek` (which must walk every sealed record), the vault
maintains a side record under the reserved namespace:

```text
namespace = "__vault__"
key       = "namespaces"
content_type = "application/vault-namespaces+json-v1"
metadata  = {
  "vault_namespaces_version": 1,
  "names": ["ns1", "ns2", ...]
}
body = (empty)
```

Every `put()` adds its namespace to this list via read-modify-write with
CAS (idempotent if the namespace is already present). On read, the
reserved namespace itself is filtered out defensively — a tampered
registry that tries to trick rotation into rewrapping the manifest
record must never succeed.

## Seal / unseal state machine

```text
    ┌────────────┐       init(password, kdf_params)        ┌────────────┐
    │  Absent    │ ──────────────────────────────────────> │ Unsealed   │
    │ (no manifest)│                                       │ (KEK in RAM)│
    └────────────┘                                         └────────────┘
          ▲                                                    │   ▲
          │ (never)                                            │   │
          │                                                    │   │
          │                 unseal(password) ✓                 ▼   │
    ┌────────────┐ ◄────────────────────────────────────── ┌────────────┐
    │  Sealed    │                                         │ Unsealed   │
    │ (no KEK)   │ ──────────────────────────────────────> │ (KEK in RAM)│
    └────────────┘             seal()                      └────────────┘
```

- **Absent**: storage contains no manifest. Only `init()` is legal.
- **Sealed**: manifest exists, KEK is not in memory. `get`/`put`/`list`
  return a `Sealed` error; `unseal()` and `stat()` are legal.
- **Unsealed**: manifest exists, KEK is in memory wrapped in
  `Zeroizing<[u8; 32]>`. All operations legal.

`seal()` zeroizes the KEK and transitions to Sealed. `Drop` on the store
also seals.

## KDF choice

Argon2id v1.3, per RFC 9106. Defaults:

| parameter       | default     | rationale                              |
|-----------------|-------------|----------------------------------------|
| time_cost       | 3           | RFC §4 "uniformly safe" profile        |
| memory_cost_kib | 65_536 (64 MiB) | RFC §4 second recommended profile  |
| parallelism     | 4           | balance latency vs. hardware           |
| tag_length      | 32          | matches ChaCha20-Poly1305 key size     |
| salt            | 16 bytes from CSPRNG | ≥ RFC-mandated minimum        |

Callers who are targeting embedded or interactive contexts can override
all of these at `init()` time. The values get recorded in the manifest
and are **not** derivable at unseal time — they live next to the salt.

## AEAD choice

XChaCha20-Poly1305 (192-bit nonce). Reasons:

- Per-record DEK means we could have used 12-byte nonces safely, but
  wrapped-DEK nonces under the KEK *will* repeat if we ever rotate keys
  in place; 24-byte random nonces give 2^96 birthday headroom without
  having to keep a monotonic counter in the manifest.
- Implementation already in this tree (`chacha20-poly1305` crate).
- Interoperable with modern ecosystems (libsodium, age).

AEAD AAD always includes the storage address to bind ciphertext to its
slot.

## Public API

```rust
/// High-level facade. Owns an Arc<dyn StorageBackend>.
pub struct SealedStore { /* private */ }

/// Knobs for a fresh init.
pub struct InitOptions {
    pub argon2id_time_cost: u32,
    pub argon2id_memory_cost_kib: u32,
    pub argon2id_parallelism: u32,
    /// None → generate a 16-byte salt from the CSPRNG.
    pub salt_override: Option<Vec<u8>>,
}

impl SealedStore {
    /// Wrap an existing backend. Does no I/O.
    pub fn new(backend: Arc<dyn StorageBackend>) -> Self;

    /// Write a fresh manifest. Fails if one already exists.
    pub fn init(&self, password: &[u8], opts: &InitOptions)
        -> Result<(), SealedStoreError>;

    /// Load the manifest, derive the KEK, verify it against the verifier.
    /// On success holds the KEK in memory until `seal()` or drop.
    pub fn unseal(&self, password: &[u8]) -> Result<(), SealedStoreError>;

    /// Wipe the KEK from memory.
    pub fn seal(&self);

    pub fn is_sealed(&self) -> bool;

    /// Write an encrypted record. Requires Unsealed.
    pub fn put(
        &self,
        namespace: &str,
        key: &str,
        plaintext: &[u8],
        if_revision: Option<Revision>,
    ) -> Result<Revision, SealedStoreError>;

    /// Read + decrypt a record. Requires Unsealed. Returns None on miss.
    pub fn get(&self, namespace: &str, key: &str)
        -> Result<Option<SealedRecord>, SealedStoreError>;

    /// Delete. Requires Unsealed.
    pub fn delete(
        &self,
        namespace: &str,
        key: &str,
        if_revision: Option<Revision>,
    ) -> Result<(), SealedStoreError>;

    /// List by prefix. Requires Unsealed. Returns *stat* views only —
    /// bodies are not decrypted until `get`.
    pub fn list(&self, namespace: &str, options: StorageListOptions)
        -> Result<Vec<SealedStat>, SealedStoreError>;

    /// Rotate the master KEK. Old password must still work; new password
    /// replaces it. Re-wraps every record's DEK; bodies are untouched.
    pub fn rotate_kek(&self, old_password: &[u8], new_password: &[u8])
        -> Result<KekRotationReport, SealedStoreError>;
}

pub struct SealedRecord {
    pub namespace: String,
    pub key: String,
    pub revision: Revision,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub plaintext: Zeroizing<Vec<u8>>,
}

pub struct SealedStat {
    pub namespace: String,
    pub key: String,
    pub revision: Revision,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub ciphertext_len: usize,
    pub kek_id: String,
}

pub enum SealedStoreError {
    AlreadyInitialized,
    NotInitialized,
    Sealed,
    BadPassword,
    Tamper { namespace: String, key: String },
    Storage(StorageError),
    Crypto(String),
    Validation { field: String, message: String },
}
```

The reserved-namespace rule: `put`/`get`/`delete`/`list` return
`Validation` if `namespace == "__vault__"` from external callers.

## Error model

- `AlreadyInitialized` — `init()` called when manifest exists.
- `NotInitialized` — any op except `init()` called when manifest absent.
- `Sealed` — data-plane op called while sealed.
- `BadPassword` — verifier did not decrypt to zeros under the derived KEK.
- `Tamper` — record body AEAD failed (ciphertext modified or AAD mismatch).
- `Storage` — underlying backend error, passed through.
- `Crypto` — low-level crypto primitive failed; message is
  low-resolution by design (no oracle-building details).
- `Validation` — caller input violated a contract.

Password checks, verifier decryption, and body tag checks all use
constant-time compares inherited from `ct-compare` / `aead_decrypt`.

## Security properties

Under the model:

- attacker sees the raw storage at rest (disk, SQLite file, memory-dump
  of the storage process)
- attacker does not have the operator password
- attacker does not have running-process memory during an unsealed
  session

…the sealed-store guarantees:

1. **Confidentiality of bodies** — ciphertexts leak only their length and
   storage address.
2. **Confidentiality of DEKs** — wrapped under the KEK with XChaCha20-P1305.
3. **Confidentiality of the KEK** — never persisted; derived from the
   password via Argon2id; held in RAM only during unsealed sessions and
   wiped on `seal()` / `drop`.
4. **Integrity** — every AEAD decryption is tag-checked. AAD binds the
   body to `(namespace, key)`.
5. **No password oracle** — the only password-derived persisted artifact
   is the verifier tag, which is a single AEAD of 16 zero bytes. The
   attacker's only attack is offline brute-force against Argon2id at the
   configured parameters.

Not guaranteed:

- **Traffic-analysis / metadata** — storage addresses, sizes, and
  timestamps are visible in cleartext (storage layer's concern).
- **Side-channels during unseal** — if the attacker observes RAM while
  the vault is unsealed, the KEK and any in-flight DEKs are exposed.
- **Denial-of-service** — a malicious storage backend can delete or
  corrupt records; the sealed-store detects it but cannot recover.

## Rotation

`rotate_kek(old_password, new_password)` procedure:

1. Unseal under `old_password` (fail → `BadPassword`).
2. Derive `KEK_new` from `new_password` + fresh 16-byte salt.
3. Build a new manifest in memory: mark the old KEK entry `retired`
   (preserving its original salt + verifier so old-password unseal
   keeps working for crash-recovery), and append a new entry
   `{ id: "kek-<n>", status: "active", salt: <new salt>,
   verifier = AEAD(KEK_new, zeros16) }`.
4. **Persist the manifest first** (CAS on its revision). After this
   point, both `KEK_old` and `KEK_new` are valid for unseal.
5. For every registered external namespace, page through every record:
   unwrap DEK under `KEK_old`, wrap under `KEK_new`, rewrite metadata
   (with `if_revision` CAS). Bodies untouched.
6. Replace the in-memory KEK with `KEK_new`; wipe `KEK_old`.

Rotation is **restartable under any crash**. Because the manifest is
persisted first, a crash mid-rewrap leaves:

- records whose `kek_id` was already rewritten to the new id — readable
  under `new_password`;
- records still carrying the old `kek_id` — still wrappable under the
  old KEK (retired but retained + verifiable), so the admin can:
  (a) unseal with `new_password` and re-run `rotate_kek(new, new)` which
  walks the still-old records and rewraps them, or
  (b) unseal with `old_password` to read them directly.

A record's `kek_id` metadata field is the source of truth for which KEK
wrapped it.

## Testing

At minimum, the test suite must cover:

- init → put → get roundtrip (multiple records)
- seal → get fails with `Sealed`; unseal → get succeeds
- wrong password → `BadPassword`; constant-ish time
- corrupt ciphertext → `Tamper`
- swap two records' bodies (keep metadata) → `Tamper`
- swap `(namespace, key)` addresses in metadata → `Tamper`
- double `init()` → `AlreadyInitialized`
- ops before `init()` → `NotInitialized`
- CAS: `put` with stale `if_revision` → `Storage(Conflict)`
- reserved namespace `__vault__` → `Validation`
- `rotate_kek`: old password still works only until rotation completes;
  then new works, old fails
- `rotate_kek` midway (simulated) → re-run finishes cleanly
- empty plaintext roundtrip
- large plaintext roundtrip (e.g. 1 MiB)

Coverage target: ≥95%.

## Out of scope

- Any notion of a *lease key* (response encryption TTL) — that is VLT02.
- Any notion of a *session key* / secure channel — that is VLT03.
- Secret-type schemas (passwords, SSH keys, TOTP, etc.) — callers define
  their own encodings on top.
- Streaming / chunked records — single-shot AEAD for now.
- Hardware-backed KEK (Secure Enclave / TPM / YubiKey) — future layer
  that replaces the KDF ceremony with a platform unlock.
