# coding_adventures_chacha20_poly1305

ChaCha20-Poly1305 authenticated encryption (RFC 8439) and
XChaCha20-Poly1305 (draft-irtf-cfrg-xchacha) implemented from scratch
in Rust.

## What It Does

This crate implements:

1. **ChaCha20** -- stream cipher using ARX (Add, Rotate, XOR) operations
2. **Poly1305** -- one-time MAC using polynomial evaluation mod 2^130 - 5
3. **AEAD** -- RFC 8439 ChaCha20-Poly1305 with a 12-byte nonce
4. **HChaCha20** -- the ChaCha20 round function as a PRF, used to derive
   a subkey from a 16-byte input nonce (draft-irtf-cfrg-xchacha §2.2)
5. **XChaCha20-Poly1305** -- the extended-nonce AEAD with a 24-byte
   nonce (draft-irtf-cfrg-xchacha §2.3), built as HChaCha20 → standard
   ChaCha20-Poly1305 with a 12-byte sub-nonce

## Fixed-memory TLS prerequisites

### ChaCha20

`chacha20_xor_fixed` is the bounded stream-cipher boundary intended for later
TLS 1.3 AEAD composition. It accepts exact caller-owned input and output
buffers, validates the complete RFC 8439 32-bit counter range before writing,
and uses one reusable wipe-on-drop keystream block. The original and working
16-word key-derived states are wiped on every return. Its rounds and arithmetic
do not branch on key or input-byte values, and the boundary does not allocate.
Callers must still prevent reuse of any key, nonce, and counter position.

This function is not authenticated encryption. It does not compute or verify
Poly1305, select a TLS cipher suite, derive or own traffic keys, construct TLS
record nonces, enforce per-key record limits, frame records, perform a TLS
handshake, validate certificates, establish trust, check revocation, perform
I/O, authorize HTTP or OAuth transport, or make TLS or HTTPS available. The
legacy allocation-returning ChaCha20, Poly1305, AEAD, and XChaCha20 APIs remain
compatibility/educational surfaces and are not promoted to the production TLS
boundary by this addition.

### Poly1305

`poly1305_mac_fixed` is the bounded one-time authenticator prerequisite for
later TLS 1.3 AEAD composition. It writes the 16-byte tag into caller-owned
fixed memory, performs no allocation, and uses the standard five 26-bit-limb
construction. Every full block follows one fixed multiplication and reduction
schedule; final conditional reduction uses masks rather than secret-dependent
control flow. Clamped key limbs, accumulator limbs, the addition pad,
multiplication products, and carry workspace share one wipe-on-drop owner, and
the reusable 16-byte message block is also scrubbed on return.

Poly1305 keys are one-time keys: callers must never reuse a key for a different
message. This primitive does not derive or reserve that key, compare tags,
encrypt or decrypt data, compose ChaCha20-Poly1305 AEAD, select a TLS cipher
suite, protect records, perform a handshake, validate certificates, establish
trust, check revocation, perform I/O, authorize HTTP or OAuth transport, or
make TLS or HTTPS available. Combined fixed-memory AEAD review remains a
separate prerequisite.

Used in TLS 1.3, WireGuard, SSH, Chrome/Android (RFC 8439), and — for
the extended-nonce flavour — libsodium `secretbox_xchacha20poly1305`,
WireGuard tunnel initiation, and most password-encrypted data at rest.

## Usage

```rust
use coding_adventures_chacha20_poly1305::{
    chacha20_encrypt, poly1305_mac, aead_encrypt, aead_decrypt,
    hchacha20_subkey, xchacha20_encrypt,
    xchacha20_poly1305_aead_encrypt, xchacha20_poly1305_aead_decrypt,
};

// ChaCha20 stream cipher
let ciphertext = chacha20_encrypt(plaintext, &key, &nonce12, counter);
let decrypted = chacha20_encrypt(&ciphertext, &key, &nonce12, counter);

// Poly1305 MAC
let tag = poly1305_mac(message, &key);

// AEAD (RFC 8439) -- 12-byte nonce
let (ct, tag) = aead_encrypt(plaintext, &key, &nonce12, aad);
let pt = aead_decrypt(&ct, &key, &nonce12, aad, &tag); // None on tamper

// XChaCha20-Poly1305 AEAD -- 24-byte nonce (safe for random nonces)
let (ct, tag) = xchacha20_poly1305_aead_encrypt(plaintext, &key, &nonce24, aad);
let pt = xchacha20_poly1305_aead_decrypt(&ct, &key, &nonce24, aad, &tag);
```

## When to pick which AEAD

| Use case                                   | Nonce width | Pick                        |
|--------------------------------------------|-------------|-----------------------------|
| Counter-based nonces (packet, session)     | 12 bytes    | `aead_encrypt`              |
| Random-nonce encryption-at-rest            | 24 bytes    | `xchacha20_poly1305_*`      |
| High-volume messaging, rotating sub-keys   | 24 bytes    | `xchacha20_poly1305_*`      |

The 12-byte AEAD's birthday bound on accidental nonce reuse is only
2^48 messages — fine for counter-driven protocols, not fine for random
nonces. XChaCha20-Poly1305's 192-bit nonce is effectively collision-
free for any realistic workload.

## How It Fits

Part of the coding-adventures cryptography stack. The sole runtime dependency
is the repository-owned `coding_adventures_zeroize` primitive used to scrub
key-derived state and keystream storage.

## Implementation Notes

- ChaCha20 uses `wrapping_add` and `rotate_left` for constant-time 32-bit operations
- Poly1305 uses a (u128, u8) pair for 130-bit accumulator with manual carry propagation
- Constant-time tag comparison to prevent timing attacks
- HChaCha20 deliberately omits the final state feed-forward; the subkey
  is drawn from rows 0 and 3 of the post-round state
- All RFC 8439 and draft-irtf-cfrg-xchacha test vectors pass exactly
- The fixed-memory ChaCha20 boundary rejects output-length and counter-range
  failures before output changes

## Portable Conformance

The test suite consumes the versioned
[`se04-xchacha20-poly1305-v1`](../../../specs/fixtures/se04-xchacha20-poly1305-v1/README.md)
fixture shared by all six D18 implementations. It proves byte-identical
HChaCha20, raw XChaCha20, and AEAD outputs plus the common authentication
failure contract. `serde_json` is a dev dependency only; the library retains
zero runtime dependencies.
