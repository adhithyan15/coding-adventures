# VLT04 — Vault Recipients (Multi-Recipient DEK Wrapping)

## Overview

The **multi-recipient wrap** layer of the Vault stack. Generalises
VLT01's single-KEK envelope to age-style N-recipient wrap-sets —
the layer that enables **sharing**, **multi-device unlock**, and
**recovery keys**.

This document specifies the `Recipient` / `Identity` traits, the
two first-party recipient kinds (`PassphraseRecipient` and
`X25519Recipient`), the wire formats, and the security properties.
Implementation lives at `code/packages/rust/vault-recipients/`.

## Why this layer exists

VLT01 wraps each per-record DEK under a single KEK. That works
for a single-user, single-device, single-passphrase deployment
and breaks the moment you want any of:

- **Sharing** (Alice creates an item; Bob reads it; the server is
  zero-knowledge).
- **Multi-device** (laptop / phone / browser extension all unwrap
  the vault key independently).
- **Recovery** (printed-at-signup recovery key as just another
  recipient on every wrap).
- **GitOps** (SOPS-style: same DEK wrapped to multiple KMS / age /
  GPG recipients).
- **Sealed Secrets** (Bitnami: cluster controller's RSA pubkey is
  a recipient).

VLT04 unifies all of these under one trait. Adding a grantee = one
more `Recipient::wrap` call appended to the wrap-set. Revoking =
re-keying the file key (a separate operation, not part of this
spec).

## Trait API

```rust
pub trait Recipient {
    fn kind(&self) -> &'static str;
    fn recipient_id(&self) -> Vec<u8>;
    fn wrap(&self, file_key: &Key) -> Result<WrappedKey, RecipientError>;
}

pub trait Identity {
    fn kind(&self) -> &'static str;
    fn recipient_id(&self) -> Vec<u8> { Vec::new() }   // default = "any"
    fn try_unwrap(&self, wrapped: &WrappedKey) -> Result<Option<Key>, RecipientError>;
}
```

`try_unwrap` returns `Ok(None)` for "not for me" (e.g. magic
mismatch — different recipient kind), `Ok(Some)` for success, and
`Err(...)` for "for me but broken" (tamper, wrong identity, etc.).

The convenience helpers `wrap_for_all(file_key, &[&Recipient])`
and `try_unwrap_any(&[&Identity], wrap_set)` cover the common
cases.

### Identity dispatch

`try_unwrap_any` uses each identity's `recipient_id()` to dispatch
only the wraps that were addressed to that identity. Two X25519
identities in the same record don't both attempt every wrap — the
wrap's stored id matches exactly one identity's id. A tampered
wrap addressed to me still propagates as `Err(UnwrapFailed)` (a
security event, not a fall-through).

The default `recipient_id()` returns an empty Vec, which falls
back to "try every wrap of matching kind." Safe only when the
identity is the unique one of its kind in the system; the
explicit-id behavior is recommended.

## First-party recipient kinds

### PassphraseRecipient

For "encrypt to a passphrase" (age `--passphrase` mode; emergency
recovery codes; CLI workflows).

Wire format:

```text
   PR_blob = magic(2) "PR" || salt(16) || nonce(24) || ct(32) || tag(16)
   total = 90 bytes
   AAD = magic
```

KEK derivation: `Argon2id(passphrase, salt, time=3, memory=64 MiB,
parallelism=4, length=32)`. Defaults follow RFC 9106 §4 baseline.

`recipient_id()` is `HKDF(salt = "PR-id-v1", ikm = passphrase,
info = "", length = 32, SHA-256)` — stable per-passphrase id so
the wrap-set layer can dispatch the right wrap to the right
identity.

### X25519Recipient + X25519Identity

age-style asymmetric wrap. The standard recipient model for
shared / multi-device vaults.

Wire format:

```text
   X1_blob = magic(2) "X1" || ephemeral_pubkey(32) || nonce(24) || ct(32) || tag(16)
   total = 106 bytes
   AAD = magic || ephemeral_pubkey || recipient_pubkey
```

Wrap algorithm:

1. Sender draws fresh ephemeral X25519 keypair `(e_sk, e_pk)`.
2. Computes `shared = X25519(e_sk, recipient_pk)`.
3. `wrap_key = HKDF(salt = e_pk || recipient_pk, ikm = shared,
   info = "VLT04-X25519-wrap-v1", length = 32, SHA-256)`.
4. AEAD-encrypts `file_key` under `wrap_key` with AAD as above.
5. Stores `e_pk` in the blob; drops `e_sk` (`Zeroizing` wipe).

Receiver: read `e_pk`, compute `shared = X25519(my_sk, e_pk)` (DH
symmetry), re-derive `wrap_key`, AEAD-decrypt. Failure = wrong
identity for this wrap or tamper.

The AAD binds the wrap to *both* the ephemeral and the recipient
pubkey, so a captured wrap can't be re-bound to a different
recipient.

`recipient_id()` returns the recipient's X25519 public key.

## Properties

### Information-flow

- A wrap-set with N recipients has `N * (90 or 106) + the
  body's-own-AEAD-overhead` bytes — linear in N, no quadratic
  blowup.
- Adding a recipient is one wrap operation (microseconds for
  X25519, ~250 ms for the Argon2id KDF in PassphraseRecipient).
- Removing a recipient requires re-keying the file_key — out of
  scope here.

### Forward secrecy of the sender

For X25519, the sender's ephemeral private key is wiped after
wrap. An attacker who later seizes the sender's storage learns
nothing about which file_keys it produced.

### Sealed Secrets / GitOps

A wrap-set persisted to a public repo is, by construction,
zero-knowledge to non-recipients. The X25519 AAD-binding stops a
malicious relay from rebinding wraps to its own pubkey.

## Threat model & test coverage

| Threat                                                                | Defence                                                                          | Test                                                                 |
|-----------------------------------------------------------------------|----------------------------------------------------------------------------------|----------------------------------------------------------------------|
| Wrong passphrase or wrong X25519 secret                               | AEAD verify; fail-closed `UnwrapFailed`                                          | `passphrase_wrong_passphrase_unwrap_fails`, `x25519_unwrap_with_wrong_identity_fails` |
| Body / nonce tamper                                                   | AEAD verify; fail-closed `UnwrapFailed`                                          | `passphrase_tamper_fails`, `x25519_unwrap_tamper_fails`              |
| Captured wrap rebound to a different recipient                        | AAD = magic ‖ e_pk ‖ recipient_pk                                                | `x25519_unwrap_with_wrong_identity_fails` (covers it via AAD)        |
| Wrong-kind blob handed to wrong identity                              | Magic prefix + `Ok(None)` (not error)                                            | `passphrase_identity_returns_none_on_x25519_blob`, `x25519_identity_returns_none_on_passphrase_blob` |
| Two same-kind identities both attempt every wrap                      | `try_unwrap_any` dispatches by `recipient_id`                                    | `multi_recipient_alice_and_bob_both_unwrap`                          |
| Third party attempts to recover any file_key                          | `recipient_id` filter rejects; `Ok(None)` returned                               | `multi_recipient_third_party_recovers_nothing`                       |
| Tampered wrap addressed to me masquerades as "not for me"             | `try_unwrap_any` propagates Err for addressed wraps                              | `try_unwrap_any_propagates_tamper_for_addressed_wrap`                |
| Sender's ephemeral private key lingering after wrap                   | `Zeroizing<[u8; 32]>` for `e_sk`; wiped on drop                                  | covered by `Zeroizing` upstream + visual review                      |
| Malformed blob (wrong length / magic)                                 | `MalformedWrappedKey`                                                            | `unwrap_too_short_blob_is_malformed`, `unwrap_truncated_*_is_malformed` |
| Attacker-controlled bytes in error logs                               | `Display` strings are static literals only                                       | `error_messages_are_static_literals`                                 |

## Out of scope (this PR)

- **RSA-OAEP recipient** — how Bitwarden / 1Password share
  between users; how Sealed Secrets works in-cluster. Future PR.
- **KMS recipients** — AWS / GCP / Azure. Future PR.
- **YubiKey-PRF recipient** — FIDO2 hmac-secret as a wrap
  primitive. Future PR.
- **Plugin recipients** — out-of-process `age-plugin-*` style.
  Future PR.
- **Revocation** — removing a recipient requires re-keying; that
  ceremony is at a higher layer.

## Citations

- age-encryption.org/v1 — the recipient model this crate adopts.
- RFC 9106 — Argon2 (used by `PassphraseRecipient`).
- RFC 7748 — X25519 (used by `X25519Recipient`).
- RFC 5869 — HKDF.
- RFC 9580 / draft — XChaCha20-Poly1305 AEAD.
- VLT00-vault-roadmap.md — VLT04 layer in the dependency chain.
