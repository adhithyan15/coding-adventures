# X25519 (Lua)

Pure Lua implementation of X25519 (RFC 7748) — elliptic curve Diffie-Hellman on Curve25519.

## What is X25519?

X25519 is a key agreement protocol based on Curve25519, a Montgomery-form elliptic curve designed by Daniel Bernstein. It's used in TLS 1.3, Signal Protocol, WireGuard, SSH, and many other systems.

## Implementation Approach

Since Lua 5.4+ only provides 64-bit integers (no arbitrary-precision), this package implements big integer arithmetic from scratch:

- **30-bit limbs**: Numbers are stored as arrays of 30-bit integers. A 256-bit number needs 9 limbs.
- **Fast reduction**: Modular reduction exploits the special form of p = 2^255 - 19, where 2^255 = 19 (mod p), to avoid expensive division.
- **Addition chain inversion**: Field inversion uses an optimized addition chain requiring only 254 squarings and 11 multiplications.

## Usage

```lua
local x25519 = require("coding_adventures.x25519")

-- Generate a keypair (private key should be 32 random bytes)
local private_key = ... -- 32 random bytes
local public_key = x25519.x25519_base(private_key)

-- Diffie-Hellman key exchange
local shared_secret = x25519.x25519(my_private, their_public)

-- Hex utilities
local hex = x25519.to_hex(shared_secret)
local bytes = x25519.from_hex(hex)
```

## API

- `x25519.x25519(scalar, u_point)` — Compute scalar * u_point. Both are 32-byte strings.
- `x25519.x25519_base(scalar)` — Compute scalar * base_point (u=9).
- `x25519.generate_keypair(private_key)` — Returns private_key, public_key.
- `x25519.from_hex(hex)` — Decode hex string to binary.
- `x25519.to_hex(bytes)` — Encode binary as hex string.

## Dependencies

None. All arithmetic is implemented from scratch using Lua's 64-bit integers.

## Running Tests

```bash
cd tests && busted . --verbose --pattern=test_
```
