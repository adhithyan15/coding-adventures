# ChaCha20-Poly1305 (Python)

A from-scratch implementation of the ChaCha20-Poly1305 AEAD cipher suite
(RFC 8439), using only ARX (Add, Rotate, XOR) operations.

## What's Inside

- **ChaCha20** stream cipher: 256-bit key, 96-bit nonce, 32-bit counter
- **Poly1305** one-time MAC: 16-byte authentication tag
- **AEAD** construction: combined authenticated encryption per RFC 8439

## Usage

```python
from coding_adventures_chacha20_poly1305 import (
    chacha20_encrypt,
    poly1305_mac,
    aead_encrypt,
    aead_decrypt,
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
```

## How It Works

ChaCha20 builds a 4x4 matrix of 32-bit words from a key, nonce, and counter,
then mixes it through 20 rounds of quarter-round operations (each using only
add, rotate, and XOR). The output is XORed with plaintext to produce ciphertext.

Poly1305 evaluates a polynomial modulo the prime 2^130 - 5 to produce a
16-byte authentication tag. Combined with ChaCha20 key derivation, this
gives the AEAD construction specified in RFC 8439.

## Building

```bash
uv venv && uv pip install -e ".[dev]"
uv run python -m pytest tests/ -v
```

## Part Of

[coding-adventures](https://github.com/adhithyan15/coding-adventures) -- a
monorepo of from-scratch implementations for learning.
