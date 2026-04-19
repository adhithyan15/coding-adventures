# coding-adventures-blake2b

A pure-Python, from-scratch implementation of the **BLAKE2b** cryptographic
hash function (RFC 7693).  Written for clarity first and cross-validated
against `hashlib.blake2b` for correctness.

## What is BLAKE2b?

BLAKE2b is a modern hash function that is:

- **Faster than MD5** on 64-bit hardware.
- **As secure as SHA-3** against all known attacks.
- **Variable output length** — any size from 1 to 64 bytes.
- **Keyed in a single pass** — replaces HMAC-SHA-512 for MAC use.
- **Parameterized** — salt and personalization fold into the initial state.

It is the hash underlying libsodium, WireGuard, Noise Protocol, IPFS
content addressing, and — within this repo — Argon2.

See the spec at [code/specs/HF06-blake2b.md](../../../specs/HF06-blake2b.md)
for the full algorithm walkthrough.

## Installation

```bash
uv pip install -e .[dev]
```

## Usage

### One-shot

```python
from coding_adventures_blake2b import blake2b, blake2b_hex

blake2b_hex(b"abc")
# 'ba80a53f981c4d0d6a2797b69f12f6e9...'

blake2b(b"abc", digest_size=32)
# 32 raw bytes
```

### Keyed (MAC mode)

```python
blake2b(b"message", key=b"shared secret", digest_size=32)
```

### Streaming

```python
from coding_adventures_blake2b import Blake2bHasher

h = Blake2bHasher(digest_size=32)
h.update(b"partial ")
h.update(b"payload")
h.hex_digest()
```

### Salt and personalization

Two applications that share a secret can domain-separate their MACs by
setting distinct 16-byte `personal` strings:

```python
blake2b(msg, key=key, personal=b"app-A-mac-v1....")
blake2b(msg, key=key, personal=b"app-B-mac-v1....")
```

## Where this fits in the stack

- **Dependencies:** none.
- **Used by:** Argon2 (forthcoming) — both for `H0` in the initial fill
  and for the BLAKE2b-long construction when the requested output
  exceeds 64 bytes.

## Scope

Sequential mode only.  Tree hashing, BLAKE2s, BLAKE2bp, BLAKE2sp,
BLAKE2Xb, and BLAKE3 are intentionally not included — see the
"Non-Goals" section of the spec.

## Running the tests

```bash
.venv/bin/python -m pytest tests/ -v
```

Tests cross-validate every one-shot and streaming path against
`hashlib.blake2b`, which wraps the reference implementation.
