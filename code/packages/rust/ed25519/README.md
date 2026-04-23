# Ed25519 Digital Signatures (Rust)

A from-scratch implementation of Ed25519 digital signatures (RFC 8032) in Rust,
using radix-2^51 field arithmetic with no external big-integer dependencies.

## What Is Ed25519?

Ed25519 is an elliptic curve digital signature algorithm (EdDSA) operating on the
twisted Edwards curve -x^2 + y^2 = 1 + d*x^2*y^2 over GF(2^255 - 19). It provides
32-byte keys, 64-byte signatures, 128-bit security, and deterministic signing.

## Usage

```rust
use coding_adventures_ed25519::{generate_keypair, sign, verify};

let seed = [0u8; 32]; // Use a CSPRNG in production!
let (public_key, secret_key) = generate_keypair(&seed);
let signature = sign(b"hello, world", &secret_key);
assert!(verify(b"hello, world", &signature, &public_key));
```

## Dependencies

- `coding_adventures_sha512` — our from-scratch SHA-512 implementation

## How It Fits

This package builds on SHA-512 and provides the signing layer needed for
protocols like SSH and TLS. It sits alongside X25519 (key exchange) in the
Curve25519 family.

## Testing

```sh
cargo test -v
```

All four RFC 8032 Section 7.1 test vectors are included, plus field arithmetic,
point operation, encoding/decoding, and rejection tests.
