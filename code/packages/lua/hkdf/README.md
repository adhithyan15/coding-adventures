# HKDF (Lua)

HMAC-based Extract-and-Expand Key Derivation Function, implementing [RFC 5869](https://www.rfc-editor.org/rfc/rfc5869).

## What is HKDF?

HKDF derives cryptographic keys from input keying material (IKM) in two stages:

1. **Extract** — condenses IKM into a fixed-length pseudorandom key (PRK) using HMAC
2. **Expand** — stretches the PRK into output keying material (OKM) of any desired length

HKDF is used in TLS 1.3, Signal Protocol, WireGuard, and many other protocols.

## API

```lua
local hkdf = require("coding_adventures.hkdf")

-- Full HKDF (extract + expand combined)
local okm = hkdf.hkdf(salt, ikm, info, length, "sha256")

-- Separate stages
local prk = hkdf.extract(salt, ikm, "sha256")
local okm = hkdf.expand(prk, info, length, "sha256")

-- Hex-output variants
local hex = hkdf.hkdf_hex(salt, ikm, info, length, "sha256")
local hex = hkdf.extract_hex(salt, ikm, "sha256")
local hex = hkdf.expand_hex(prk, info, length, "sha256")
```

All functions accept `"sha256"` (default) or `"sha512"` as the hash algorithm.

## Dependencies

- `coding-adventures-hmac` (which depends on SHA-256, SHA-512, MD5, SHA-1)

## Running Tests

```bash
cd tests && busted . --verbose --pattern=test_
```
