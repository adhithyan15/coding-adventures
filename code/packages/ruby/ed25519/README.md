# coding_adventures_ed25519

Ed25519 digital signature algorithm (RFC 8032) implemented from scratch using Ruby's native arbitrary-precision integers.

## What is Ed25519?

Ed25519 is a high-speed, high-security digital signature scheme built on the twisted Edwards curve `-x^2 + y^2 = 1 + d*x^2*y^2` over the prime field GF(2^255 - 19). It provides:

- **128-bit security** (equivalent to ~3072-bit RSA)
- **Deterministic signatures** (no random nonce needed)
- **32-byte public keys** and **64-byte signatures**
- **Complete addition formula** (resistant to timing attacks)

## How It Fits in the Stack

This package depends on `coding_adventures_sha512` for:
- Key derivation (SHA-512 of seed)
- Deterministic nonce generation
- Challenge hash computation

## Usage

```ruby
require "coding_adventures_ed25519"

# Generate a keypair from a 32-byte seed
seed = SecureRandom.random_bytes(32)
public_key, secret_key = CodingAdventures::Ed25519.generate_keypair(seed)

# Sign a message
message = "Hello, world!"
signature = CodingAdventures::Ed25519.sign(message, secret_key)

# Verify the signature
valid = CodingAdventures::Ed25519.verify(message, signature, public_key)
puts valid  # => true
```

## API

### `CodingAdventures::Ed25519.generate_keypair(seed) -> [public_key, secret_key]`

Generate a keypair from a 32-byte seed. Returns [32-byte public key, 64-byte secret key].

### `CodingAdventures::Ed25519.sign(message, secret_key) -> signature`

Sign a message with the secret key. Returns a 64-byte deterministic signature.

### `CodingAdventures::Ed25519.verify(message, signature, public_key) -> bool`

Verify a signature against a message and public key. Returns true if valid.

## Test Vectors

Tested against RFC 8032 Section 7.1 test vectors, verified with Node.js built-in Ed25519.
