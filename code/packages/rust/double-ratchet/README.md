# coding_adventures_double_ratchet

Signal Double Ratchet Algorithm — forward-secret, break-in-recovering message
encryption. Implemented from scratch in Rust with no external crypto crates.

## What This Does

The Double Ratchet gives two security properties simultaneously:

1. **Forward secrecy** — compromising today's keys does not reveal past messages.
   Each message uses a fresh key derived from the chain; old keys are deleted.

2. **Break-in recovery** — after a compromise, the session "heals" automatically.
   Every new DH ratchet step (triggered by a reply) introduces fresh randomness
   that an attacker who captured old state cannot predict.

## The Two Ratchets

### Symmetric KDF Chain

```
CK₀ → CK₁ → CK₂ → …
 ↓      ↓      ↓
MK₁   MK₂   MK₃
```

`CKₙ₊₁ = HMAC-SHA256(CKₙ, 0x01)`  
`MKₙ   = HMAC-SHA256(CKₙ, 0x02)`

### DH Ratchet (Root Chain)

```
RK ──DH(new_keys)──► RK', CKs_or_CKr
```

`RK', CK = HKDF(salt=RK, ikm=DH_output, info="WhisperRatchet", len=64)`

## Usage

```rust
use coding_adventures_double_ratchet::{
    generate_ratchet_keypair, ratchet_init_alice, ratchet_init_bob,
    ratchet_encrypt, ratchet_decrypt,
};

// Shared key from X3DH; Bob's ratchet public key from his prekey bundle
let sk = [0u8; 32]; // from X3DH
let bob_kp = generate_ratchet_keypair();
let bob_pub = bob_kp.public;

let mut alice = ratchet_init_alice(&sk, &bob_pub);
let mut bob   = ratchet_init_bob(&sk, bob_kp);

// Alice → Bob
let msg = ratchet_encrypt(&mut alice, b"hello", b"").unwrap();
let pt  = ratchet_decrypt(&mut bob,   &msg,    b"").unwrap();
assert_eq!(pt, b"hello");

// Bob → Alice (triggers a DH ratchet step)
let reply = ratchet_encrypt(&mut bob,   b"world", b"").unwrap();
let pt2   = ratchet_decrypt(&mut alice, &reply,   b"").unwrap();
assert_eq!(pt2, b"world");
```

## Out-of-Order Delivery

Messages that arrive out of order are handled automatically. The ratchet skips
ahead, storing unused message keys in a `HashMap<(dh_pub, n), mk>`. When the
missing messages finally arrive, their stored keys are used for decryption.

A `MAX_SKIP = 1000` limit prevents DoS attacks where an adversary sends a
message with a huge counter, forcing the receiver to derive thousands of keys.

## Key Zeroization

- `KeyPair` implements `Zeroize` and `Drop` — secret zeroed on drop.
- `RatchetState` implements `Drop` — root key, chain keys, and all skipped
  message keys are wiped before deallocation.
- DH outputs and HKDF OKMs use `Zeroizing<>` RAII wrappers.

## Dependencies

- `coding_adventures_curve25519` — X25519 for DH ratchet steps
- `coding_adventures_hkdf` — HKDF-SHA256 for root chain KDF
- `coding_adventures_hmac` — HMAC-SHA256 for symmetric chain KDF
- `coding_adventures_chacha20_poly1305` — AEAD for message encryption
- `coding_adventures_zeroize` — Zeroize trait and Zeroizing wrapper
- `getrandom` — OS entropy for ratchet key generation

## References

- [Signal Double Ratchet Specification](https://signal.org/docs/specifications/doubleratchet/)
- Trevor Perrin, Moxie Marlinspike — "The Double Ratchet Algorithm" (2016)
