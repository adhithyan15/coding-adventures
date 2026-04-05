# coding-adventures-sha512

SHA-512 cryptographic hash function (FIPS 180-4) implemented from scratch in Python.

## What Is SHA-512?

SHA-512 is the 64-bit sibling of SHA-256 in the SHA-2 family. It produces a 512-bit (64-byte) digest using 8 x 64-bit state words and 80 rounds of compression. On 64-bit platforms, SHA-512 is often faster than SHA-256 because it processes 128-byte blocks using native 64-bit arithmetic.

## How It Fits

This package is part of the `coding-adventures` monorepo hash function collection. It sits alongside MD5, SHA-1, and SHA-256 as a from-scratch implementation intended for learning. The code uses literate programming style with extensive inline explanations.

## Usage

```python
from coding_adventures_sha512 import sha512, sha512_hex, SHA512Hasher

# One-shot hashing
digest = sha512(b"abc")          # 64 bytes
hex_str = sha512_hex(b"abc")     # 128-char hex string

# Streaming (for large data)
h = SHA512Hasher()
h.update(b"ab")
h.update(b"c")
h.hex_digest()  # same result as sha512_hex(b"abc")

# Copy for prefix sharing
h = SHA512Hasher()
h.update(b"common_prefix")
h1 = h.copy()
h1.update(b"suffix_a")
h2 = h.copy()
h2.update(b"suffix_b")
```

## API

| Function | Returns | Description |
|----------|---------|-------------|
| `sha512(data)` | `bytes` | 64-byte digest |
| `sha512_hex(data)` | `str` | 128-char lowercase hex string |
| `SHA512Hasher()` | object | Streaming hasher |
| `.update(data)` | `self` | Feed bytes (chainable) |
| `.digest()` | `bytes` | Get 64-byte result (non-destructive) |
| `.hex_digest()` | `str` | Get 128-char hex (non-destructive) |
| `.copy()` | `SHA512Hasher` | Deep copy of hasher state |

## Development

```bash
uv venv && uv pip install -e ".[dev]"
.venv/bin/python -m pytest tests/ -v
```
