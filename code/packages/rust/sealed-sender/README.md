# coding_adventures_sealed_sender

Signal Sealed Sender — sender-anonymous message delivery. The final layer of
the Signal cryptographic stack, hiding sender identity from the server.
Implemented from scratch in Rust with no external crypto crates.

## What This Does

Normally a message has a visible sender field the server can log. Sealed Sender
wraps the entire message — including the sender's identity — in an ephemeral
ECDH encryption keyed to the **recipient's** identity key. The server sees only:

- A routing token (hash of the recipient's key, not their name)
- The encrypted envelope (cannot be opened without the recipient's private key)

## Protocol Layers

```
┌─────────────────────────────────────────────────────────────┐
│  Outer: Ephemeral-ECDH sealed envelope                      │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  SenderCertificate (server-signed Ed25519)            │  │
│  │  ┌────────────────────────────────────────────────┐   │  │
│  │  │  Double Ratchet message (forward-secret AEAD)  │   │  │
│  │  └────────────────────────────────────────────────┘   │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Usage

```rust
use coding_adventures_sealed_sender::{
    generate_identity_keypair, generate_prekey_pair,
    create_prekey_bundle, x3dh_send, x3dh_receive,
    ratchet_init_alice, ratchet_init_bob, KeyPair,
    issue_sender_certificate, sealed_send, sealed_receive,
};
use coding_adventures_ed25519::generate_keypair as ed_keypair;

// Key setup
let alice_ik = generate_identity_keypair();
let bob_ik   = generate_identity_keypair();
let bob_spk  = generate_prekey_pair();
let bundle   = create_prekey_bundle(&bob_ik, &bob_spk, 1, None);

// X3DH session establishment
let alice_x3dh = x3dh_send(&alice_ik, &bundle).unwrap();
let bob_sk     = x3dh_receive(&bob_ik, &bob_spk, None,
    &alice_ik.x25519_public, &alice_x3dh.ephemeral_public).unwrap();

// Double Ratchet initialization
let mut alice_r = ratchet_init_alice(&alice_x3dh.shared_key, &bob_spk.public);
let mut bob_r   = ratchet_init_bob(&bob_sk, KeyPair::from_secret(*bob_spk.secret()));

// Server issues a certificate for Alice
let (server_vk, server_sk) = ed_keypair(&[42u8; 32]);
let cert = issue_sender_certificate(
    [1u8; 16], 1, alice_ik.x25519_public,
    now_ms + 86_400_000,    // +1 day expiry
    &server_sk,
);

// Alice seals a message to Bob
let sealed = sealed_send(&cert, &mut alice_r, &bob_ik.x25519_public, b"hello", b"").unwrap();

// Bob unseals and verifies
let (plaintext, sender_cert) =
    sealed_receive(&bob_ik, &sealed, &mut bob_r, &server_vk, now_ms, b"").unwrap();
assert_eq!(plaintext, b"hello");
assert_eq!(sender_cert.ik_public, alice_ik.x25519_public);
```

## Wire Format

### SenderCertificate (124 bytes)

| Field | Offset | Length | Description |
|-------|--------|--------|-------------|
| uuid | 0 | 16 | Sender's UUID v4 |
| device_id | 16 | 4 | u32 LE device number |
| ik_public | 20 | 32 | X25519 identity public key |
| expires_at | 52 | 8 | u64 LE Unix ms expiry |
| server_sig | 60 | 64 | Ed25519 sig over bytes[0..60] |

### SealedMessage envelope

```
eph_pub[0..32]     ephemeral X25519 public key
ciphertext[32..]   ChaCha20-Poly1305 encrypted inner payload (last 16 bytes = tag)
```

### Inner payload

```
cert_len (4 LE)    always CERT_LEN = 124
cert_bytes         SenderCertificate
header_bytes       Double Ratchet MessageHeader (HEADER_LEN = 40)
ct_len (4 LE)      length of ratchet ciphertext
ct                 Double Ratchet ciphertext + 16-byte Poly1305 tag
```

## Key Derivation

```
DH_out  = X25519(eph_secret, recipient_IK_x25519)
okm     = HKDF(salt=0×32, ikm=DH_out, info="sealed-sender-v1", len=44)
enc_key = okm[0..32]
nonce   = okm[32..44]
```

AAD for the envelope AEAD = `eph_pub` (binds ciphertext to the key exchange).

## Security Properties

- **Sender anonymity** — server cannot identify the sender without the
  recipient's private key.
- **Certificate expiry** — checked with `now_ms >= cert.expires_at` to
  correctly reject expired certificates.
- **Overflow-safe decode** — all payload offset arithmetic uses checked
  addition to prevent integer overflow on malformed input.
- **Secret zeroization** — ephemeral keys and DH outputs are held in
  `Zeroizing<>` RAII wrappers.

## Dependencies

- `coding_adventures_x3dh` — X3DH key agreement (re-exported)
- `coding_adventures_double_ratchet` — Double Ratchet encryption (re-exported)
- `coding_adventures_curve25519` — X25519 ECDH
- `coding_adventures_ed25519` — Ed25519 signing/verification
- `coding_adventures_hkdf` — HKDF-SHA256
- `coding_adventures_chacha20_poly1305` — AEAD
- `coding_adventures_zeroize` — Zeroize trait
- `getrandom` — OS entropy

## References

- [Signal Sealed Sender Blog Post](https://signal.org/blog/sealed-sender/)
- Trevor Perrin — "Sealed Sender for Signal" (2018)
