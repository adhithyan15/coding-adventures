# ChaCha20-Poly1305 (Python)

A dependency-free, from-scratch implementation of ChaCha20-Poly1305 (RFC 8439)
and XChaCha20-Poly1305 (SE04, pinned to `draft-irtf-cfrg-xchacha-03`).

## What's Inside

- **ChaCha20** stream cipher: 256-bit key, 96-bit nonce, 32-bit counter
- **Poly1305** one-time MAC: 16-byte authentication tag
- **AEAD** construction: combined authenticated encryption per RFC 8439
- **HChaCha20** subkey derivation: 256-bit key and 128-bit input nonce
- **XChaCha20** and **XChaCha20-Poly1305**: 192-bit nonces

## Usage

```python
from coding_adventures_chacha20_poly1305 import (
    chacha20_encrypt,
    poly1305_mac,
    aead_encrypt,
    aead_decrypt,
    hchacha20_subkey,
    xchacha20_encrypt,
    xchacha20_poly1305_aead_encrypt,
    xchacha20_poly1305_aead_decrypt,
)

# Stream cipher
key = bytes(32)       # 256-bit key
nonce = bytes(12)     # 96-bit nonce
ct = chacha20_encrypt(b"hello", key, nonce, counter=0)

# One-time MAC
tag = poly1305_mac(b"message", key)

# Authenticated encryption
ct, tag = aead_encrypt(b"secret", key, nonce, aad=b"metadata")
pt = aead_decrypt(ct, key, nonce, aad=b"metadata", tag=tag)

# Extended-nonce authenticated encryption
nonce24 = bytes(24)   # Must still be unique for each key
ct, tag = xchacha20_poly1305_aead_encrypt(
    b"secret", key, nonce24, aad=b"metadata",
)
pt = xchacha20_poly1305_aead_decrypt(
    ct, key, nonce24, aad=b"metadata", tag=tag,
)
```

## How It Works

ChaCha20 builds a 4x4 matrix of 32-bit words from a key, nonce, and counter,
then mixes it through 20 rounds of quarter-round operations (each using only
add, rotate, and XOR). The output is XORed with plaintext to produce ciphertext.

Poly1305 evaluates a polynomial modulo the prime 2^130 - 5 to produce a
16-byte authentication tag. Combined with ChaCha20 key derivation, this
gives the AEAD construction specified in RFC 8439.

XChaCha20 runs the ChaCha20 round function without feed-forward to derive a
subkey from the first 16 nonce bytes. It then delegates to the same RFC 8439
implementation with the nonce `0x00000000 || nonce24[16:24]`. The 24-byte
nonce makes random collisions negligible at realistic volumes, but nonce
uniqueness remains mandatory: this construction is not nonce-misuse resistant.

This package is educational hand-rolled cryptography. It matches the official
RFC 8439 and pinned XChaCha Internet-Draft vectors, but applications should use
a professionally audited cryptographic library for production secrets.

## Building

```bash
uv venv && uv pip install -e ".[dev]"
uv run python -m pytest tests/ -v
```

## Portable Conformance

The test suite consumes the versioned
[`se04-xchacha20-poly1305-v1`](../../../specs/fixtures/se04-xchacha20-poly1305-v1/README.md)
fixture shared by all six D18 implementations. It proves byte-identical
HChaCha20, raw XChaCha20, and AEAD outputs plus the common authentication
failure contract. Fixture parsing uses only Python's standard library.

## Part Of

[coding-adventures](https://github.com/adhithyan15/coding-adventures) -- a
monorepo of from-scratch implementations for learning.
