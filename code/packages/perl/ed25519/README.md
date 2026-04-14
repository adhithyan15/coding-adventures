# Ed25519 (Perl)

Pure Perl implementation of Ed25519 digital signatures as defined in [RFC 8032](https://datatracker.ietf.org/doc/html/rfc8032).

## What Is Ed25519?

Ed25519 is an elliptic curve digital signature algorithm (EdDSA) that uses the twisted Edwards curve `-x^2 + y^2 = 1 + d*x^2*y^2` over the prime field GF(2^255 - 19). It provides:

- **32-byte public keys** and **64-byte signatures**
- **128-bit security level** against all known attacks
- **Deterministic signatures** -- no random nonce needed
- **Fast** signing and verification

## Dependencies

- `CodingAdventures::Sha512` -- SHA-512 hash function (used internally for key derivation, nonce generation, and challenge hashing)

## Usage

```perl
use CodingAdventures::Ed25519;

# Generate a keypair from a 32-byte seed
my $seed = "\0" x 32;  # use a real random seed!
my ($public_key, $secret_key) = CodingAdventures::Ed25519::generate_keypair($seed);

# Sign a message
my $signature = CodingAdventures::Ed25519::sign("Hello, world!", $secret_key);

# Verify a signature
my $valid = CodingAdventures::Ed25519::verify("Hello, world!", $signature, $public_key);
die "Invalid!" unless $valid;
```

## Implementation Notes

Uses Perl's core `Math::BigInt` module for arbitrary-precision integer arithmetic. Points are represented in extended twisted Edwards coordinates (X, Y, Z, T) for unified addition formulas.

## API

- `generate_keypair($seed)` -- Returns `($public_key, $secret_key)` (32 bytes, 64 bytes)
- `sign($message, $secret_key)` -- Returns 64-byte signature
- `verify($message, $signature, $public_key)` -- Returns 1 (valid) or 0 (invalid)
- `from_hex($hex)` -- Decode hex string to binary
- `to_hex($binary)` -- Encode binary to hex string
