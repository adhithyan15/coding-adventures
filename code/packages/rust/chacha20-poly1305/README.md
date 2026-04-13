# coding_adventures_chacha20_poly1305

ChaCha20-Poly1305 authenticated encryption (RFC 8439) implemented from scratch in Rust.

## What It Does

This crate implements the ChaCha20-Poly1305 AEAD cipher suite, combining:

1. **ChaCha20** -- a stream cipher using ARX (Add, Rotate, XOR) operations
2. **Poly1305** -- a one-time MAC using polynomial evaluation mod 2^130 - 5
3. **AEAD** -- authenticated encryption with associated data

Used in TLS 1.3, WireGuard, SSH, and Chrome/Android as the primary alternative to AES-GCM.

## Usage

```rust
use coding_adventures_chacha20_poly1305::{chacha20_encrypt, poly1305_mac, aead_encrypt, aead_decrypt};

// ChaCha20 stream cipher
let ciphertext = chacha20_encrypt(plaintext, &key, &nonce, counter);
let decrypted = chacha20_encrypt(&ciphertext, &key, &nonce, counter);

// Poly1305 MAC
let tag = poly1305_mac(message, &key);

// AEAD (recommended)
let (ct, tag) = aead_encrypt(plaintext, &key, &nonce, aad);
let pt = aead_decrypt(&ct, &key, &nonce, aad, &tag); // Returns None on tamper
```

## How It Fits

Part of the coding-adventures cryptography stack. Self-contained -- no runtime dependencies.

## Implementation Notes

- ChaCha20 uses `wrapping_add` and `rotate_left` for constant-time 32-bit operations
- Poly1305 uses a (u128, u8) pair for 130-bit accumulator with manual carry propagation
- Constant-time tag comparison to prevent timing attacks
- All RFC 8439 test vectors pass exactly
