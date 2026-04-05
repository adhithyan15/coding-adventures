# SE03 — ChaCha20-Poly1305

## Overview

ChaCha20-Poly1305 (RFC 8439) is an authenticated encryption scheme
combining the ChaCha20 stream cipher with the Poly1305 MAC. Designed by
Daniel J. Bernstein, it's the primary alternative to AES-GCM and is
used in TLS 1.3, WireGuard, SSH, and Chrome/Android.

### Why ChaCha20 Instead of AES?

- **No hardware dependency:** AES-GCM is fast on CPUs with AES-NI
  instructions, but slow in software (and vulnerable to cache-timing
  attacks). ChaCha20 is fast in pure software on any CPU.
- **Simpler:** ChaCha20 uses only ARX operations (Add, Rotate, XOR) —
  no lookup tables, no GF(2^8) arithmetic.
- **Mobile-friendly:** ARM processors without AES instructions
  (pre-ARMv8) get 3× better performance with ChaCha20.

## Algorithm

### ChaCha20 Stream Cipher

State: 4×4 matrix of 32-bit words initialized from:
- Constants: "expand 32-byte k" (4 words)
- Key: 256 bits (8 words)
- Counter: 32 bits (1 word)
- Nonce: 96 bits (3 words)

Quarter round: a, b, c, d = QR(a, b, c, d)
```
a += b; d ^= a; d <<<= 16
c += d; b ^= c; b <<<= 12
a += b; d ^= a; d <<<= 8
c += d; b ^= c; b <<<= 7
```

20 rounds (10 column rounds + 10 diagonal rounds), then add original
state. Output: 64-byte keystream block.

### Poly1305 MAC

One-time authenticator producing a 16-byte tag.
- Key: 32 bytes (r: 16 bytes clamped, s: 16 bytes)
- Processes 16-byte blocks as numbers in GF(2^130 - 5)
- Accumulate: acc = ((acc + block) * r) mod (2^130 - 5)
- Finalize: tag = (acc + s) mod 2^128

### Combined AEAD

```
1. Generate Poly1305 key: first 32 bytes of ChaCha20(key, nonce, counter=0)
2. Encrypt plaintext with ChaCha20(key, nonce, counter=1)
3. Construct Poly1305 input: AAD || pad || ciphertext || pad || len(AAD) || len(CT)
4. Compute tag = Poly1305(poly_key, input)
```

## Interface Contract

| Function | Signature | Description |
|----------|-----------|-------------|
| `chacha20_encrypt` | `(plaintext, key_32, nonce_12, counter) -> ciphertext` | Stream cipher (XOR). |
| `poly1305_mac` | `(message, key_32) -> tag_16` | One-time MAC. |
| `aead_encrypt` | `(plaintext, key, nonce, aad) -> (ciphertext, tag)` | Authenticated encryption. |
| `aead_decrypt` | `(ciphertext, key, nonce, aad, tag) -> plaintext` | Authenticated decryption (fails if tag invalid). |

## Test Vectors (RFC 8439)

```
# ChaCha20
Key:   000102...1f (32 bytes)
Nonce: 000000000000004a00000000
Counter: 1
Plaintext: "Ladies and Gentlemen of the class of '99: If I could..."
# See RFC 8439 Section 2.4.2 for full ciphertext

# Poly1305
Key: 85d6be7857556d337f4452fe42d506a80103808afb0db2fd4abff6af4149f51b
Message: "Cryptographic Forum Research Group"
Tag: a8061dc1305136c6c22b8baf0c0127a9

# AEAD
Key: 808182...9f (32 bytes)
Nonce: 070000004041424344454647
AAD: 50515253c0c1c2c3c4c5c6c7
Plaintext: "Ladies and Gentlemen of the class of '99: If I could..."
Tag: 1ae10b594f09e26a7e902ecbd0600691
```

## Package Matrix

Same 9 languages, in `chacha20-poly1305/` directories.

**Dependencies:** None. Self-contained (only ARX operations).
