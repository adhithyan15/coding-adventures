# coding_adventures_hkdf

HKDF (HMAC-based Extract-and-Expand Key Derivation Function) implemented from scratch following RFC 5869.

## What Is HKDF?

HKDF derives one or more cryptographically strong keys from input keying material (IKM). It is the standard key derivation function used in TLS 1.3, Signal Protocol, WireGuard, and many other modern protocols.

## Usage

```rust
use coding_adventures_hkdf::{hkdf, hkdf_extract, hkdf_expand, HashAlgorithm};

// Combined extract-then-expand (most common usage)
let okm = hkdf(b"salt", b"ikm", b"info", 32, HashAlgorithm::Sha256).unwrap();

// Separate extract and expand
let prk = hkdf_extract(b"salt", b"ikm", HashAlgorithm::Sha256);
let okm = hkdf_expand(&prk, b"info", 32, HashAlgorithm::Sha256).unwrap();

// SHA-512 variant
let okm = hkdf(b"salt", b"ikm", b"info", 64, HashAlgorithm::Sha512).unwrap();
```

## API

- `hkdf_extract(salt, ikm, algorithm) -> Vec<u8>` — Extract phase
- `hkdf_expand(prk, info, length, algorithm) -> Result<Vec<u8>, HkdfError>` — Expand phase
- `hkdf(salt, ikm, info, length, algorithm) -> Result<Vec<u8>, HkdfError>` — Combined

## Dependencies

- `coding_adventures_hmac` (which depends on `coding_adventures_sha256`, `coding_adventures_sha512`, etc.)

## How It Fits in the Stack

```
HKDF (this crate)
  └── HMAC
        ├── SHA-256
        └── SHA-512
```
