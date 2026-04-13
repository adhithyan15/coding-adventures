# coding_adventures_x25519

X25519 Elliptic Curve Diffie-Hellman key agreement (RFC 7748), implemented from scratch with no external dependencies.

## What is X25519?

X25519 is the Diffie-Hellman function on Curve25519. It allows two parties to establish a shared secret over an insecure channel. Used in TLS 1.3, SSH, Signal, WireGuard, and many other protocols.

## Usage

```ruby
require "coding_adventures_x25519"

# Generate keypairs (in practice, use SecureRandom.random_bytes(32).bytes)
alice_private = Array.new(32) { rand(256) }
alice_public = CodingAdventures::X25519.generate_keypair(alice_private)

bob_private = Array.new(32) { rand(256) }
bob_public = CodingAdventures::X25519.generate_keypair(bob_private)

# Compute shared secret
alice_shared = CodingAdventures::X25519.x25519(alice_private, bob_public)
bob_shared = CodingAdventures::X25519.x25519(bob_private, alice_public)
# alice_shared == bob_shared
```

## API

- `CodingAdventures::X25519.x25519(scalar, u)` -- scalar multiplication on Curve25519
- `CodingAdventures::X25519.x25519_base(scalar)` -- multiply by base point (u=9)
- `CodingAdventures::X25519.generate_keypair(private_key)` -- alias for x25519_base

All inputs/outputs are 32-element arrays of integers (0-255) in little-endian encoding.

## Implementation details

- Field arithmetic over GF(2^255-19) using Ruby's native arbitrary-precision integers
- Montgomery ladder for scalar multiplication
- Scalar clamping per RFC 7748
- Fermat's little theorem for modular inversion (via Integer#pow)

## Part of coding-adventures

This package is part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) educational computing stack.
