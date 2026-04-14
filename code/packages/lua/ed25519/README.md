# Ed25519 (Lua)

Pure Lua implementation of Ed25519 digital signatures as defined in [RFC 8032](https://datatracker.ietf.org/doc/html/rfc8032).

## What Is Ed25519?

Ed25519 is an elliptic curve digital signature algorithm (EdDSA) that uses the twisted Edwards curve `-x^2 + y^2 = 1 + d*x^2*y^2` over the prime field GF(2^255 - 19). It provides:

- **32-byte public keys** and **64-byte signatures**
- **128-bit security level** against all known attacks
- **Deterministic signatures** -- no random nonce needed (prevents Sony PS3-style failures)
- **Fast** signing and verification

## Dependencies

- `coding-adventures-sha512` -- SHA-512 hash function (used internally for key derivation, nonce generation, and challenge hashing)

## Usage

```lua
local ed25519 = require("coding_adventures.ed25519")

-- Generate a keypair from a 32-byte seed
local seed = string.rep("\0", 32)  -- use a real random seed!
local public_key, secret_key = ed25519.generate_keypair(seed)

-- Sign a message
local signature = ed25519.sign("Hello, world!", secret_key)

-- Verify a signature
local valid = ed25519.verify("Hello, world!", signature, public_key)
assert(valid)
```

## Implementation Notes

Since Lua 5.4 has only 64-bit integers (no arbitrary-precision arithmetic), this implementation uses arrays of 30-bit "limbs" for big integer arithmetic. Three arithmetic layers are needed:

1. **Big integers**: schoolbook multiplication with 30-bit limbs
2. **Field arithmetic** (mod p = 2^255-19): fast reduction using 2^255 = 19 (mod p)
3. **Scalar arithmetic** (mod L): binary long division for reducing 512-bit SHA-512 outputs

Points are represented in extended twisted Edwards coordinates (X, Y, Z, T) for unified addition formulas.

## API

- `generate_keypair(seed)` -- Returns `public_key, secret_key` (32 bytes, 64 bytes)
- `sign(message, secret_key)` -- Returns 64-byte signature
- `verify(message, signature, public_key)` -- Returns boolean
- `from_hex(hex)` -- Decode hex string to binary
- `to_hex(binary)` -- Encode binary to hex string
