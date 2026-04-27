# coding_adventures_x3dh

Extended Triple Diffie-Hellman (X3DH) key agreement — the asynchronous session
establishment protocol used by Signal, WhatsApp, and many other secure messaging
applications. Implemented from scratch in Rust with no external crypto crates.

## What This Does

X3DH allows Alice to establish a shared secret with Bob **even when Bob is offline**.
Bob pre-publishes a bundle of public keys; Alice uses them to compute a shared
secret and sends her ephemeral public key so Bob can reproduce the same secret later.

## The Four DH Operations

```
DH1 = X25519(IK_A_dh,  SPK_B)    long-term identity   × medium-term signed prekey
DH2 = X25519(EK_A,     IK_B_dh)  ephemeral            × long-term identity
DH3 = X25519(EK_A,     SPK_B)    ephemeral             × medium-term signed prekey
DH4 = X25519(EK_A,     OPK_B)    ephemeral             × one-time prekey (optional)

KM  = F ‖ DH1 ‖ DH2 ‖ DH3 [‖ DH4]   (F = 0xFF × 32 bytes)
SK  = HKDF(salt=F, ikm=KM, info="WhisperText", L=32)
```

## Usage

```rust
use coding_adventures_x3dh::{
    generate_identity_keypair, generate_prekey_pair,
    create_prekey_bundle, x3dh_send, x3dh_receive,
};

// Bob publishes his key bundle
let bob_ik  = generate_identity_keypair();
let bob_spk = generate_prekey_pair();
let bob_opk = generate_prekey_pair();
let bundle  = create_prekey_bundle(&bob_ik, &bob_spk, 1, Some((&bob_opk, 1)));

// Alice establishes the session
let alice_ik  = generate_identity_keypair();
let alice_out = x3dh_send(&alice_ik, &bundle).unwrap();
// alice_out.shared_key  → 32-byte shared secret for Double Ratchet
// alice_out.ephemeral_public → sent to Bob in the initial message

// Bob derives the same shared secret
let sk = x3dh_receive(
    &bob_ik, &bob_spk, Some(&bob_opk),
    &alice_ik.x25519_public,
    &alice_out.ephemeral_public,
).unwrap();
assert_eq!(alice_out.shared_key, sk);
```

## Security Properties

- **Authentication** — DH1 binds Alice's long-term key; DH2 binds Bob's long-term key.
- **Forward secrecy** — DH3 uses Alice's ephemeral key; SPK rotation limits exposure.
- **One-time prekeys** — DH4 prevents replay attacks; each OPK can only be used once.
- **Deniability** — the protocol produces no signatures from either party on the session key.

## Key Zeroization

All secret scalars are held in `Zeroizing<>` RAII wrappers or structs with
`impl Drop`. Intermediate DH outputs (dh1–dh4) and the IKM buffer are wiped
from stack memory as soon as they are no longer needed.

## Dependencies

- `coding_adventures_curve25519` — X25519 scalar multiplication
- `coding_adventures_ed25519` — Ed25519 signing / verification
- `coding_adventures_hkdf` — HKDF-SHA256 key derivation
- `coding_adventures_zeroize` — Zeroize trait and Zeroizing wrapper
- `getrandom` — OS entropy source

## References

- [Signal X3DH Specification](https://signal.org/docs/specifications/x3dh/)
- RFC 7748 — Elliptic Curves for Security (X25519)
- RFC 8032 — Edwards-Curve Digital Signature Algorithm (Ed25519)
