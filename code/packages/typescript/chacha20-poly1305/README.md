# @coding-adventures/chacha20-poly1305

ChaCha20-Poly1305 authenticated encryption (RFC 8439) implemented from scratch in TypeScript.

## What It Does

This package implements the ChaCha20-Poly1305 AEAD cipher suite, combining:

1. **ChaCha20** -- a stream cipher using ARX (Add, Rotate, XOR) operations
2. **Poly1305** -- a one-time MAC using polynomial evaluation mod 2^130 - 5
3. **AEAD** -- authenticated encryption with associated data

Used in TLS 1.3, WireGuard, SSH, and Chrome/Android as the primary alternative to AES-GCM.

## Usage

```typescript
import { chacha20Encrypt, poly1305Mac, aeadEncrypt, aeadDecrypt } from "@coding-adventures/chacha20-poly1305";

// ChaCha20 stream cipher
const ciphertext = chacha20Encrypt(plaintext, key32, nonce12, counter);
const decrypted = chacha20Encrypt(ciphertext, key32, nonce12, counter); // XOR is self-inverse

// Poly1305 MAC
const tag = poly1305Mac(message, key32);

// AEAD (recommended for most uses)
const [ct, tag] = aeadEncrypt(plaintext, key32, nonce12, aad);
const pt = aeadDecrypt(ct, key32, nonce12, aad, tag); // throws on tamper
```

## How It Fits

Part of the coding-adventures cryptography stack. Self-contained -- no dependencies on other packages. Uses native BigInt for Poly1305's modular arithmetic.

## Implementation Notes

- ChaCha20 uses 32-bit wrapping arithmetic via `>>> 0`
- Poly1305 uses JavaScript's native `BigInt` for 130-bit arithmetic
- Constant-time tag comparison to prevent timing attacks
- All RFC 8439 test vectors pass exactly
