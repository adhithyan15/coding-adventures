# HKDF

HKDF (HMAC-based Extract-and-Expand Key Derivation Function, RFC 5869) implemented from scratch in Swift.

## What It Does

HKDF transforms raw input keying material (from Diffie-Hellman exchanges, passwords, etc.) into cryptographically strong keys. It operates in two phases:

1. **Extract** -- compress non-uniform input into a fixed-size pseudorandom key (PRK)
2. **Expand** -- stretch the PRK into any number of output bytes, with domain separation via an "info" string

HKDF is the key derivation function used in TLS 1.3, Signal Protocol, WireGuard, and the Web Crypto API.

## Usage

```swift
import HKDF

// Full HKDF: extract-then-expand in one call
let key = try hkdf(salt: salt, ikm: sharedSecret, info: info, length: 32)

// Or use the phases separately for multiple derived keys
let prk = hkdfExtract(salt: salt, ikm: sharedSecret)
let encKey = try hkdfExpand(prk: prk, info: "enc".data(using: .utf8)!, length: 32)
let macKey = try hkdfExpand(prk: prk, info: "mac".data(using: .utf8)!, length: 32)

// SHA-512 variant
let key512 = try hkdf(salt: salt, ikm: ikm, info: info, length: 64, hash: .sha512)
```

## How It Fits

This package builds on the HMAC package, which uses the SHA256 and SHA512 packages. It sits between HMAC (authentication) and higher-level protocols like TLS key schedules and PBKDF2.

## API

- `hkdfExtract(salt:ikm:hash:)` -- Extract phase. Returns PRK (HashLen bytes).
- `hkdfExpand(prk:info:length:hash:)` -- Expand phase. Returns OKM (length bytes). Throws on invalid length.
- `hkdf(salt:ikm:info:length:hash:)` -- Combined extract-then-expand. Throws on invalid length.

Hash defaults to `.sha256`. Also accepts `.sha512`.
