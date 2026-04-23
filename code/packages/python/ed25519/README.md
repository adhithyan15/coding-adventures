# Ed25519 Digital Signatures (RFC 8032)

A from-scratch implementation of Ed25519 digital signatures using the twisted Edwards curve over GF(2^255-19).

## What It Does

Ed25519 provides three operations:

- **Key generation**: Derive a 32-byte public key from a 32-byte seed
- **Signing**: Produce a 64-byte deterministic signature for a message
- **Verification**: Check that a signature is valid for a given message and public key

## How It Fits in the Stack

This package depends on `coding-adventures-sha512` for hashing. Ed25519 uses SHA-512 internally to derive secret scalars, generate deterministic nonces, and compute challenge values.

The underlying curve (Curve25519) is the same one used by the `x25519` package for key exchange, but in its twisted Edwards form rather than Montgomery form.

## Usage

```python
from coding_adventures_ed25519 import generate_keypair, sign, verify
import os

# Generate a keypair from a random seed
seed = os.urandom(32)
public_key, secret_key = generate_keypair(seed)

# Sign a message
message = b"Hello, Ed25519!"
signature = sign(message, secret_key)

# Verify the signature
assert verify(message, signature, public_key)
```

## Algorithm Details

Ed25519 uses the twisted Edwards curve `-x^2 + y^2 = 1 + d*x^2*y^2` over GF(2^255-19) with extended coordinates for efficient point arithmetic. Signing is deterministic (no random nonce), preventing catastrophic nonce-reuse vulnerabilities.

## Running Tests

```bash
uv venv --quiet --clear
uv pip install -e ../sha512 -e .[dev] --quiet
.venv/bin/python -m pytest tests/ -v
```
