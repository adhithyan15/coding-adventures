# `coding_adventures_vault_secure_channel` — VLT-CH

Signal-protocol-style **continuous-key-rotation channel** for the
Vault stack. Composes the existing `coding_adventures_x3dh`
(initial key agreement) and `coding_adventures_double_ratchet`
(per-message DH + KDF chain) crates into one ergonomic wrapper.

**The "channel takeover" guarantee.** A snapshot of channel state
at time T cannot decrypt:
- Any message sent *before* T (forward secrecy — the message keys
  are deleted as soon as the messages are processed).
- Any message sent *after* the next DH ratchet step (post-
  compromise security — the new chain is rooted in fresh DH
  output the attacker doesn't have).

## Quick example

```rust
use coding_adventures_vault_secure_channel::{
    ChannelInitiator, ChannelResponder,
};
use coding_adventures_x3dh::{
    create_prekey_bundle, generate_identity_keypair, generate_prekey_pair,
};

// Bob publishes a PreKeyBundle out of band.
let bob_ik   = generate_identity_keypair();
let bob_spk  = generate_prekey_pair();
let bob_opk  = generate_prekey_pair();
let bundle   = create_prekey_bundle(&bob_ik, &bob_spk, 1, Some((&bob_opk, 7)));

// Alice opens the channel and sends the first message.
let alice_ik = generate_identity_keypair();
let (mut alice, first) = ChannelInitiator::open(
    &alice_ik, &bundle, b"hello bob", b"vault/123",
)?;

// Bob accepts and gets the first plaintext.
let (mut bob, first_pt) = ChannelResponder::accept(
    &first, &bob_ik, &bob_spk, Some(&bob_opk),
    &alice_ik.x25519_public, b"vault/123",
)?;
assert_eq!(&first_pt[..], b"hello bob");

// Both sides now exchange messages indefinitely; every message
// uses a fresh derived key, every reply triggers a DH ratchet step.
let wire = bob.send(b"hello alice", b"vault/123")?;
let pt   = alice.receive(&wire, b"vault/123")?;
```

## Wire format

```text
First message: "C1" || ek_pub(32) || dr_header(40) || ct_len(4 BE) || ct
Subsequent:    "CN"             || dr_header(40) || ct_len(4 BE) || ct
```

Caller-supplied `aad` is passed through to the ratchet AEAD (binds
the ciphertext to application context).

## Where it fits

```text
   vault-records (VLT02)  →  bytes
                              │
       wrap-set (VLT04)  ←  file_key  →  ciphertext
                              │
              ┌───────────────▼──────────────────┐
              │  vault-secure-channel (VLT-CH)  ◄│  THIS CRATE
              │  X3DH initial agreement +        │
              │  Double Ratchet per-message      │
              └───────────────┬──────────────────┘
                              │
               ┌──────────────▼──────────────┐
               │  vault-transport (VLT11)    │
               │  HTTP / gRPC / CLI / etc.   │
               └─────────────────────────────┘
```

See [`VLT00-vault-roadmap.md`](../../../specs/VLT00-vault-roadmap.md)
and [`VLT-CH-vault-secure-channel.md`](../../../specs/VLT-CH-vault-secure-channel.md).
