# ChaCha20Poly1305

ChaCha20-Poly1305 authenticated encryption (RFC 8439) implemented from scratch in Swift.

## What It Does

This package implements the ChaCha20-Poly1305 AEAD cipher suite, combining:

1. **ChaCha20** -- a stream cipher using ARX (Add, Rotate, XOR) operations
2. **Poly1305** -- a one-time MAC using polynomial evaluation mod 2^130 - 5
3. **AEAD** -- authenticated encryption with associated data

Used in TLS 1.3, WireGuard, SSH, and Chrome/Android as the primary alternative to AES-GCM.

## Usage

```swift
import ChaCha20Poly1305

// ChaCha20 stream cipher
let ciphertext = ChaCha20Poly1305.chacha20Encrypt(plaintext: data, key: key, nonce: nonce, counter: 1)
let decrypted = ChaCha20Poly1305.chacha20Encrypt(plaintext: ciphertext, key: key, nonce: nonce, counter: 1)

// Poly1305 MAC
let tag = ChaCha20Poly1305.poly1305Mac(message: data, key: key)

// AEAD (recommended)
let (ct, tag) = ChaCha20Poly1305.aeadEncrypt(plaintext: data, key: key, nonce: nonce, aad: header)
let pt = ChaCha20Poly1305.aeadDecrypt(ciphertext: ct, key: key, nonce: nonce, aad: header, tag: tag)
```

## How It Fits

Part of the coding-adventures cryptography stack. Self-contained -- no dependencies on other packages.

## Implementation Notes

- ChaCha20 uses `&+` for wrapping 32-bit addition and manual bit rotation
- Poly1305 uses a radix-2^26 representation with 5 limbs for 130-bit arithmetic
- Constant-time tag comparison to prevent timing attacks
- All RFC 8439 test vectors pass exactly
- Compatible with Swift 6 strict concurrency
