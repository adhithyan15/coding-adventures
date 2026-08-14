# ChaCha20-Poly1305 (Go)

A dependency-free, from-scratch implementation of ChaCha20-Poly1305 (RFC 8439)
and XChaCha20-Poly1305 (SE04, pinned to `draft-irtf-cfrg-xchacha-03`).

## What's Inside

- **ChaCha20** stream cipher: 256-bit key, 96-bit nonce, 32-bit counter
- **Poly1305** one-time MAC: 16-byte authentication tag (uses math/big for 130-bit arithmetic)
- **AEAD** construction: combined authenticated encryption per RFC 8439
- **HChaCha20** subkey derivation: 256-bit key and 128-bit input nonce
- **XChaCha20** and **XChaCha20-Poly1305**: 192-bit nonces

## Usage

```go
import chacha20poly1305 "github.com/adhithyan15/coding-adventures/code/packages/go/chacha20-poly1305"

// Stream cipher
ct, err := chacha20poly1305.ChaCha20Encrypt(plaintext, key, nonce, 0)

// One-time MAC
tag, err := chacha20poly1305.Poly1305Mac(message, key)

// Authenticated encryption
ct, tag, err := chacha20poly1305.AEADEncrypt(plaintext, key, nonce, aad)
pt, err := chacha20poly1305.AEADDecrypt(ct, key, nonce, aad, tag)

// Extended-nonce authenticated encryption
ct, tag, err = chacha20poly1305.XChaCha20Poly1305AEADEncrypt(plaintext, key, nonce24, aad)
pt, err = chacha20poly1305.XChaCha20Poly1305AEADDecrypt(ct, key, nonce24, aad, tag)
```

XChaCha20 derives a subkey from the first 16 nonce bytes and delegates to the
same RFC 8439 implementation with `0x00000000 || nonce24[16:24]`. A 24-byte
nonce makes random collisions negligible at realistic volumes, but complete
nonces must still be unique for each key; this construction is not
nonce-misuse resistant.

This package is educational hand-rolled cryptography. It matches the official
RFC 8439 and pinned XChaCha Internet-Draft vectors, but production applications
should use a professionally audited cryptographic library.

## Building

```bash
go test ./... -v -cover
```

## Portable Conformance

The test suite consumes the versioned
[`se04-xchacha20-poly1305-v1`](../../../specs/fixtures/se04-xchacha20-poly1305-v1/README.md)
fixture shared by all six D18 implementations. It proves byte-identical
HChaCha20, raw XChaCha20, and AEAD outputs plus the common authentication
failure contract using only Go's standard library.

## Part Of

[coding-adventures](https://github.com/adhithyan15/coding-adventures) -- a
monorepo of from-scratch implementations for learning.
