# Ed25519 (Elixir)

Pure Elixir implementation of Ed25519 digital signatures as defined in [RFC 8032](https://datatracker.ietf.org/doc/html/rfc8032).

## What Is Ed25519?

Ed25519 is an elliptic curve digital signature algorithm (EdDSA) that uses the twisted Edwards curve `-x^2 + y^2 = 1 + d*x^2*y^2` over the prime field GF(2^255 - 19). It provides:

- **32-byte public keys** and **64-byte signatures**
- **128-bit security level** against all known attacks
- **Deterministic signatures** -- no random nonce needed (prevents Sony PS3-style failures)
- **Fast** signing and verification

## Dependencies

- `coding_adventures_sha512` -- SHA-512 hash function (used internally for key derivation, nonce generation, and challenge hashing)

## Usage

```elixir
alias CodingAdventures.Ed25519

# Generate a keypair from a 32-byte seed
seed = :crypto.strong_rand_bytes(32)
{public_key, secret_key} = Ed25519.generate_keypair(seed)

# Sign a message
signature = Ed25519.sign("Hello, world!", secret_key)

# Verify a signature
true = Ed25519.verify("Hello, world!", signature, public_key)
```

## Implementation Notes

Elixir has native arbitrary-precision integers, so all field and scalar arithmetic uses standard operators with modular reduction. No limb-based representation is needed.

Points are represented in extended twisted Edwards coordinates (X, Y, Z, T) for unified addition formulas. The implementation uses:

1. **Field arithmetic** (mod p = 2^255-19): standard modular operations
2. **Scalar arithmetic** (mod L): reduction of 512-bit SHA-512 outputs modulo the group order
3. **Point operations**: unified addition and doubling in extended coordinates

## API

- `generate_keypair(seed)` -- Returns `{public_key, secret_key}` (32 bytes, 64 bytes)
- `sign(message, secret_key)` -- Returns 64-byte signature
- `verify(message, signature, public_key)` -- Returns boolean
- `from_hex(hex)` -- Decode hex string to binary
- `to_hex(binary)` -- Encode binary to hex string
