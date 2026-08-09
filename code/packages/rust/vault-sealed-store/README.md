# vault-sealed-store

Rust implementation of **VLT01** (`code/specs/VLT01-vault-sealed-store.md`)
— the at-rest encryption layer of the Vault stack.

This crate turns any `storage_core::StorageBackend` into an
**encrypted-secrets store** whose plaintext is only readable while a
verified password-derived or custody-supplied KEK is loaded in memory. Hosts can also read a
sealed-safe status summary for initialization, KEK history, and namespace
registry health without decrypting record bodies. Unsealed hosts can also
inspect redacted per-record envelope summaries that expose algorithms and
byte counts without copying ciphertext, wrapped DEKs, nonces, or tags into
logs.

## What this layer does

```text
          ┌────────────────────────────┐
          │      sealed-store          │   ← this crate (VLT01)
          │  envelope encryption +     │
          │  seal/unseal ceremony      │
          └──────────────┬─────────────┘
                         │
                         ▼
          ┌────────────────────────────┐
          │     StorageBackend         │   (storage-core)
          │  opaque bytes + metadata   │
          └────────────────────────────┘
```

Envelope encryption means: every secret gets a fresh 32-byte Data
Encryption Key (DEK) from the CSPRNG. The DEK encrypts the plaintext
with XChaCha20-Poly1305. The DEK itself is then wrapped under a master
Key Encryption Key (KEK), either derived from the operator password via
Argon2id or injected after a caller-owned custody ceremony. Rotating a
password-derived KEK is O(records × 32 bytes), not
O(records × body size).

## Usage

```rust
use std::sync::Arc;
use coding_adventures_vault_sealed_store::{SealedStore, InitOptions};
use storage_core::{InMemoryStorageBackend, StorageBackend};

let backend: Arc<dyn StorageBackend> = Arc::new(InMemoryStorageBackend::new());
backend.initialize()?;
let vault = SealedStore::new(backend.clone());

vault.init(b"correct horse battery staple", &InitOptions::default())?;
vault.put("passwords", "github.com", b"my-pat-token", None)?;
vault.put_if_absent("passwords", "new-service", b"one-time-secret")?;

let secret = vault.get("passwords", "github.com")?.unwrap();
assert_eq!(&*secret.plaintext, b"my-pat-token");

vault.seal(); // wipes the KEK from RAM
```

`put_if_absent` applies the backend's atomic absence condition after envelope
encryption, so a generated credential address can never overwrite an existing
record. Failed conditional writes still drop and zeroize the fresh cleartext
DEK before returning.

For a product vault, generate one random root KEK, wrap it with a
`vault-key-custody` provider, and inject the unwrapped key material:

```rust
# use std::sync::Arc;
# use coding_adventures_vault_sealed_store::SealedStore;
# use storage_core::{InMemoryStorageBackend, StorageBackend};
# let backend: Arc<dyn StorageBackend> = Arc::new(InMemoryStorageBackend::new());
# backend.initialize()?;
# let vault = SealedStore::new(backend);
let root_kek: [u8; 32] = coding_adventures_csprng::random_array()?;
vault.init_with_kek(&root_kek)?;
vault.seal();
vault.unseal_with_kek(&root_kek)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

The manifest records that this KEK was injected, but never persists the
key itself. Password unseal ignores injected entries and injected unseal
ignores password-derived entries. Legacy manifests without a source marker
continue to be treated as password-derived.

## Threat model

See the spec for the full argument. In short:

- Confidentiality of bodies and DEKs is guaranteed against an attacker
  who sees the storage at rest but not the unsealed process's memory.
- Integrity is enforced via AEAD; AAD binds each ciphertext to its
  storage address so records cannot be swapped.
- The only key-derived persisted artifact is a verifier AEAD of 16 zero
  bytes. For password-derived KEKs, an attacker's only path is offline
  brute force against Argon2id at the configured parameters.

## Dependencies

- `storage-core` — the trait this layer sits on top of.
- `coding_adventures_argon2id` — KDF.
- `coding_adventures_chacha20_poly1305` — XChaCha20-Poly1305 AEAD.
- `coding_adventures_csprng` — OS entropy source.
- `coding_adventures_zeroize` — key-wiping primitives.
- `coding-adventures-json-value` — metadata encoding.
