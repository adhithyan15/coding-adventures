# `coding_adventures_vault_recipients` — VLT04

Multi-recipient DEK wrapping for the Vault stack. Generalises
VLT01's single-KEK envelope to N-recipient wrap-sets — the layer
that enables **sharing**, **multi-device unlock**, and **recovery
keys** without re-encrypting record bodies.

age-shaped: each `Recipient` produces an opaque wrap blob of the
file key for itself; the corresponding `Identity` unwraps it.
Records carry a wrap-set `Vec<(recipient_id, wrapped)>` so the
same file key is encrypted once but accessible to many recipients.

## Quick example

```rust
use coding_adventures_vault_recipients::{
    fresh_file_key, wrap_for_all, try_unwrap_any,
    PassphraseRecipient, X25519Identity, Recipient, Identity,
};

// 1. Alice and Bob each have an X25519 identity.
let alice = X25519Identity::generate()?;
let bob   = X25519Identity::generate()?;

// 2. Plus a recovery passphrase known to both.
let recovery = PassphraseRecipient::with_default_params(b"emergency words".to_vec())?;

// 3. Wrap a fresh file_key to all three recipients.
let fk = fresh_file_key()?;
let wrap_set = wrap_for_all(
    &fk,
    &[&alice.recipient() as &dyn Recipient,
      &bob.recipient(),
      &recovery],
)?;

// 4. Persist (wrap_set, ciphertext-of-record-under-fk) somewhere.

// 5. Later, any of the three can recover.
let recovered = try_unwrap_any(&[&alice as &dyn Identity], &wrap_set)?
    .expect("Alice can recover");
```

## What's included (v0.1)

- **`Recipient`** + **`Identity`** traits.
- **`PassphraseRecipient`** — Argon2id KDF + XChaCha20-Poly1305
  AEAD wrap. 90-byte blob.
- **`X25519Recipient`** / **`X25519Identity`** — age-style
  asymmetric wrap. Fresh ephemeral keypair per wrap, ECDH-HKDF-AEAD
  derivation. 106-byte blob.
- **`wrap_for_all`** / **`try_unwrap_any`** convenience helpers.

## Wire formats

```text
PassphraseRecipient blob (90 bytes):
   magic(2) "PR" || salt(16) || nonce(24) || ct(32) || tag(16)
   AAD = magic

X25519Recipient blob (106 bytes):
   magic(2) "X1" || ephemeral_pubkey(32) || nonce(24) || ct(32) || tag(16)
   AAD = magic || ephemeral_pubkey || recipient_pubkey
```

The X25519 AAD binds the wrap to *both* the ephemeral and the
recipient pubkey, so a captured wrap can't be silently re-bound
to a different recipient by a malicious relay.

## Identity dispatch in `try_unwrap_any`

For wraps of the same kind addressed to different recipients (e.g.
two X25519 identities in the same record), `try_unwrap_any` uses
the wrap's stored `recipient_id` to dispatch only to the matching
identity. This avoids the "Bob's identity attempts Alice's wrap,
fails AEAD, propagates as tamper" failure mode.

A tampered wrap that *was* addressed to me still propagates as
`Err(UnwrapFailed)` — that's a security event, not a fall-through.

## Future work (separate PRs)

- **RSA-OAEP recipient** — how Bitwarden / 1Password share between
  users today; how Bitnami Sealed Secrets work in-cluster.
- **KMS recipients** — AWS KMS, GCP Cloud KMS, Azure Key Vault
  Managed HSM. Wrap is `kms:Encrypt(file_key)` with no local key
  material.
- **YubiKey-PRF recipient** — FIDO2 hmac-secret as a wrap
  primitive.
- **Plugin recipients** — out-of-process wrap/unwrap binaries
  (`age-plugin-*` style).

## Where it fits

```text
                     ┌──────────────────────────────┐
                     │  vault-records  (VLT02)      │
                     └──────────────┬───────────────┘
                                    │
                     ┌──────────────▼───────────────┐
                     │  vault-recipients (VLT04)  ◄ │  THIS CRATE
                     │  N-recipient wrap-sets       │
                     └──────────────┬───────────────┘
                                    │  file_key
                                    ▼
                     ┌──────────────────────────────┐
                     │  vault-sealed-store (VLT01) │
                     │  envelope encryption         │
                     └──────────────────────────────┘
```

See [`VLT00-vault-roadmap.md`](../../../specs/VLT00-vault-roadmap.md)
and [`VLT04-vault-recipients.md`](../../../specs/VLT04-vault-recipients.md).
