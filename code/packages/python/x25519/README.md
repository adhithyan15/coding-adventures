# X25519 — Elliptic Curve Diffie-Hellman (RFC 7748)

A pure-Python, zero-dependency implementation of the X25519 key agreement protocol. All field arithmetic over GF(2^255 - 19) is implemented from scratch.

## What is X25519?

X25519 is the most widely used key agreement protocol on the internet. It enables two parties to derive a shared secret over an insecure channel — the foundation of TLS 1.3, SSH, Signal, and WireGuard.

It performs scalar multiplication on Curve25519 (a Montgomery curve) using only x-coordinates and the Montgomery ladder algorithm for constant-time execution.

## Usage

```python
import os
from coding_adventures_x25519 import x25519, x25519_base, generate_keypair

# Generate keypairs
alice_private = os.urandom(32)
alice_public = generate_keypair(alice_private)

bob_private = os.urandom(32)
bob_public = generate_keypair(bob_private)

# Derive shared secret (both compute the same value)
shared_a = x25519(alice_private, bob_public)
shared_b = x25519(bob_private, alice_public)
assert shared_a == shared_b
```

## API

- `x25519(scalar, u_coordinate) -> bytes` — Core scalar multiplication
- `x25519_base(scalar) -> bytes` — Multiply by base point (u=9)
- `generate_keypair(private_key) -> bytes` — Derive public key from private key

All inputs and outputs are 32 bytes, little-endian encoded.

## How It Works

1. **Field arithmetic** — Add, subtract, multiply, square, and invert over GF(2^255 - 19)
2. **Scalar clamping** — Clear low 3 bits (cofactor), clear bit 255, set bit 254
3. **Montgomery ladder** — 255 constant-time iterations using projective coordinates
4. **Final inversion** — Convert from projective (X:Z) to affine (X/Z) coordinates

## Testing

```bash
pytest tests/ -v
```

Tests include all RFC 7748 test vectors plus the iterated 1000-round test.
