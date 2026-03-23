# sha1 (Python)

SHA-1 cryptographic hash function (FIPS 180-4) implemented from scratch in Python.

## What It Does

SHA-1 takes any sequence of bytes and produces a fixed-size 20-byte (160-bit) digest.
The same input always yields the same digest. Change one bit of input and the entire
digest changes — the avalanche effect. This package implements SHA-1 from scratch,
without using Python's `hashlib`, so every step of the algorithm is visible.

## How It Fits in the Stack

This package is part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures)
monorepo. SHA-1 is a prerequisite for the UUID v5 package, which uses SHA-1 to produce
deterministic name-based UUIDs from a namespace and a name string.

SHA-1 is no longer considered collision-resistant (the SHAttered attack, 2017) and should
not be used for security-critical purposes. It remains valid for UUID v5 and Git object IDs.

## Algorithm Overview

SHA-1 uses the Merkle-Damgård construction:

```
message ──► [pad] ──► block₀ ──► block₁ ──► ... ──► 20-byte digest
                           │           │
                   [H₀..H₄]──►compress──►compress──►...
```

1. **Pad** the message to a multiple of 64 bytes (append `0x80`, zeros, then bit length).
2. **Process** each 64-byte block through 80 rounds of bit mixing using four auxiliary functions.
3. **Finalize** by outputting the five 32-bit state words as big-endian bytes.

## Usage

```python
from sha1 import sha1, sha1_hex, SHA1

# One-shot
digest = sha1(b"abc")            # bytes, length 20
hex_str = sha1_hex(b"abc")       # "a9993e364706816aba3e25717850c26c9cd0d89d"

# Streaming
h = SHA1()
h.update(b"ab")
h.update(b"c")
print(h.hexdigest())             # "a9993e364706816aba3e25717850c26c9cd0d89d"

# Copy for prefix hashing
h2 = h.copy()
h2.update(b" world")
```

## FIPS 180-4 Test Vectors

```python
assert sha1_hex(b"") == "da39a3ee5e6b4b0d3255bfef95601890afd80709"
assert sha1_hex(b"abc") == "a9993e364706816aba3e25717850c26c9cd0d89d"
assert sha1_hex(b"a" * 1_000_000) == "34aa973cd4c4daa4f61eeb2bdbad27316534016f"
```

## Development

```bash
# On Linux/Mac
bash BUILD

# On Windows
bash BUILD_windows
```

Tests: 37 tests, 100% coverage.
