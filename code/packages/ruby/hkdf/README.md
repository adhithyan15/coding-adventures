# coding_adventures_hkdf

HKDF (HMAC-based Extract-and-Expand Key Derivation Function, RFC 5869) implemented from scratch in Ruby.

## What It Does

HKDF transforms raw input keying material (from Diffie-Hellman exchanges, passwords, etc.) into cryptographically strong keys. It operates in two phases:

1. **Extract** -- compress non-uniform input into a fixed-size pseudorandom key (PRK)
2. **Expand** -- stretch the PRK into any number of output bytes, with domain separation via an "info" string

HKDF is the key derivation function used in TLS 1.3, Signal Protocol, WireGuard, and the Web Crypto API.

## Usage

```ruby
require 'coding_adventures_hkdf'

# Full HKDF: extract-then-expand in one call
key = CodingAdventures::HKDF.hkdf(salt, ikm, info, 32) # 32-byte key

# Or use the phases separately for multiple derived keys
prk = CodingAdventures::HKDF.hkdf_extract(salt, ikm)
enc_key = CodingAdventures::HKDF.hkdf_expand(prk, "enc", 32)
mac_key = CodingAdventures::HKDF.hkdf_expand(prk, "mac", 32)

# SHA-512 variant
key512 = CodingAdventures::HKDF.hkdf(salt, ikm, info, 64, "sha512")
```

## How It Fits

This package builds on `coding_adventures_hmac`, which uses the SHA-256 and SHA-512 packages. It sits between HMAC (authentication) and higher-level protocols like TLS key schedules and PBKDF2.

## API

- `CodingAdventures::HKDF.hkdf_extract(salt, ikm, hash="sha256")` -- Extract phase
- `CodingAdventures::HKDF.hkdf_expand(prk, info, length, hash="sha256")` -- Expand phase
- `CodingAdventures::HKDF.hkdf(salt, ikm, info, length, hash="sha256")` -- Combined
