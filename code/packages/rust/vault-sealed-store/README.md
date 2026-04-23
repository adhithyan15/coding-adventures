# vault-sealed-store

Rust implementation of **VLT01** (`code/specs/VLT01-vault-sealed-store.md`)
— the at-rest encryption layer of the Vault stack.

This crate turns any `storage_core::StorageBackend` into an
**encrypted-secrets store** whose plaintext is only readable while a
correct operator password is loaded in memory.

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
Key Encryption Key (KEK) derived from the operator password via
Argon2id. Rotating the KEK is O(records × 32 bytes), not
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

let secret = vault.get("passwords", "github.com")?.unwrap();
assert_eq!(&*secret.plaintext, b"my-pat-token");

vault.seal(); // wipes the KEK from RAM
```

## Threat model

See the spec for the full argument. In short:

- Confidentiality of bodies and DEKs is guaranteed against an attacker
  who sees the storage at rest but not the unsealed process's memory.
- Integrity is enforced via AEAD; AAD binds each ciphertext to its
  storage address so records cannot be swapped.
- The only password-derived persisted artifact is a verifier AEAD of 16
  zero bytes — an attacker's only path is offline brute force against
  Argon2id at the configured parameters.

## Dependencies

- `storage-core` — the trait this layer sits on top of.
- `coding_adventures_argon2id` — KDF.
- `coding_adventures_chacha20_poly1305` — XChaCha20-Poly1305 AEAD.
- `coding_adventures_csprng` — OS entropy source.
- `coding_adventures_zeroize` — key-wiping primitives.
- `coding-adventures-json-value` — metadata encoding.
