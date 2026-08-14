# @coding-adventures/chacha20-poly1305

ChaCha20-Poly1305 authenticated encryption (RFC 8439) and the extended-nonce
XChaCha20-Poly1305 construction pinned by SE04, implemented from scratch in
TypeScript.

## What It Does

This package implements the ChaCha20-Poly1305 AEAD cipher suite, combining:

1. **ChaCha20** -- a stream cipher using ARX (Add, Rotate, XOR) operations
2. **Poly1305** -- a one-time MAC using polynomial evaluation mod 2^130 - 5
3. **AEAD** -- authenticated encryption with associated data
4. **HChaCha20 and XChaCha20** -- subkey derivation and a 192-bit nonce form

Used in TLS 1.3, WireGuard, SSH, and Chrome/Android as the primary alternative to AES-GCM.

## Usage

```typescript
import {
  chacha20Encrypt,
  poly1305Mac,
  aeadEncrypt,
  aeadDecrypt,
  hchacha20Subkey,
  xchacha20Encrypt,
  xchacha20Poly1305Encrypt,
  xchacha20Poly1305Decrypt,
} from "@coding-adventures/chacha20-poly1305";

// ChaCha20 stream cipher
const ciphertext = chacha20Encrypt(plaintext, key32, nonce12, counter);
const decrypted = chacha20Encrypt(ciphertext, key32, nonce12, counter); // XOR is self-inverse

// Poly1305 MAC
const tag = poly1305Mac(message, key32);

// AEAD (recommended for most uses)
const [ct, tag] = aeadEncrypt(plaintext, key32, nonce12, aad);
const pt = aeadDecrypt(ct, key32, nonce12, aad, tag); // throws on tamper

// SE04 HChaCha20 and raw XChaCha20
const subkey = hchacha20Subkey(key32, nonce16);
const rawCt = xchacha20Encrypt(plaintext, key32, nonce24, counter);

// SE04 XChaCha20-Poly1305 (recommended when a 24-byte nonce is available)
const [xct, xtag] = xchacha20Poly1305Encrypt(plaintext, key32, nonce24, aad);
const xpt = xchacha20Poly1305Decrypt(xct, key32, nonce24, aad, xtag);
```

## How It Fits

Part of the coding-adventures cryptography stack. Self-contained -- no dependencies on other packages. Uses native BigInt for Poly1305's modular arithmetic.

## Implementation Notes

- ChaCha20 uses 32-bit wrapping arithmetic via `>>> 0`
- Poly1305 uses JavaScript's native `BigInt` for 130-bit arithmetic
- Constant-time tag comparison to prevent timing attacks
- All RFC 8439 test vectors pass exactly
- The HChaCha20 and XChaCha20-Poly1305 vectors from
  `draft-irtf-cfrg-xchacha-03` pass exactly. The draft is a pinned construction
  reference, not a final IETF standard.
- A random 24-byte nonce makes accidental collisions negligible at realistic
  volumes, but the complete nonce must still be unique for each key.
- Derived subkeys are overwritten on a best-effort basis. JavaScript runtimes
  do not guarantee that optimized or copied buffers are erased from memory.

## Portable Conformance

The test suite consumes the versioned
[`se04-xchacha20-poly1305-v1`](../../../specs/fixtures/se04-xchacha20-poly1305-v1/README.md)
fixture shared by all six D18 implementations. It proves byte-identical
HChaCha20, raw XChaCha20, and AEAD outputs plus the common authentication
failure contract. Fixture parsing is confined to Vitest and adds no runtime
dependency.

This package is educational, hand-written cryptographic code. Prefer a mature,
audited cryptography library for production secrets.
