# coding-adventures-hkdf

HKDF (HMAC-based Extract-and-Expand Key Derivation Function) implemented from scratch following RFC 5869.

## What Is HKDF?

HKDF derives one or more cryptographically strong keys from input keying material (IKM). It is the standard key derivation function used in TLS 1.3, Signal Protocol, WireGuard, and many other modern protocols.

HKDF works in two phases:

1. **Extract**: Concentrates entropy from IKM into a fixed-length pseudorandom key (PRK) using HMAC
2. **Expand**: Derives as many output bytes as needed from the PRK using chained HMAC calls

## Installation

```bash
pip install coding-adventures-hkdf
```

## Usage

```python
from coding_adventures_hkdf import hkdf, hkdf_extract, hkdf_expand

# Combined extract-then-expand (most common usage)
salt = b"my-salt"
ikm = b"input-keying-material"
info = b"application-context"
okm = hkdf(salt, ikm, info, length=32)

# Separate extract and expand
prk = hkdf_extract(salt, ikm)
okm = hkdf_expand(prk, info, length=32)

# SHA-512 variant
okm = hkdf(salt, ikm, info, length=64, hash="sha512")
```

## API

- `hkdf_extract(salt, ikm, hash="sha256") -> bytes` — Extract phase
- `hkdf_expand(prk, info, length, hash="sha256") -> bytes` — Expand phase
- `hkdf(salt, ikm, info, length, hash="sha256") -> bytes` — Combined

## Supported Hash Functions

| Algorithm | Hash Length | Max Output |
|-----------|-----------|------------|
| SHA-256   | 32 bytes  | 8160 bytes |
| SHA-512   | 64 bytes  | 16320 bytes|

## Dependencies

- `coding-adventures-hmac` (which depends on `coding-adventures-sha256`, `coding-adventures-sha512`, etc.)

## How It Fits in the Stack

This package sits on top of the HMAC package, which itself builds on the SHA-256 and SHA-512 hash implementations:

```
HKDF (this package)
  └── HMAC
        ├── SHA-256
        └── SHA-512
```

HKDF is a building block for higher-level protocols like TLS 1.3 key scheduling and PBKDF2.
