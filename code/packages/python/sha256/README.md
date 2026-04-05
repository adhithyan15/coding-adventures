# coding-adventures-sha256

SHA-256 cryptographic hash function (FIPS 180-4) implemented from scratch in Python.

## What Is SHA-256?

SHA-256 is a cryptographic hash function from the SHA-2 family that produces a 256-bit (32-byte) digest. It is the workhorse of modern cryptography, used in TLS, Bitcoin, git, code signing, and password hashing.

This package implements SHA-256 from first principles with no dependencies on `hashlib` or any other cryptographic library. Every step of the algorithm is visible and explained using literate programming style.

## How It Fits in the Stack

This is package HF03 in the coding-adventures monorepo. It builds on the same Merkle-Damgard construction used in the SHA-1 package (HF02), but with a wider state (8 words instead of 5), more complex message schedule, and stronger auxiliary functions.

## Installation

```bash
pip install -e ".[dev]"
```

## Usage

### One-shot hashing

```python
from coding_adventures_sha256 import sha256, sha256_hex

digest = sha256(b"hello world")        # 32 bytes
hex_str = sha256_hex(b"hello world")   # 64-char hex string
```

### Streaming (chunked) hashing

```python
from coding_adventures_sha256 import SHA256Hasher

h = SHA256Hasher()
h.update(b"hello ")
h.update(b"world")
print(h.hex_digest())  # same as sha256_hex(b"hello world")
```

### Branching with copy()

```python
h = SHA256Hasher()
h.update(b"common prefix")
h1 = h.copy()
h2 = h.copy()
h1.update(b" branch A")
h2.update(b" branch B")
# h1 and h2 have different digests
```

## API

| Function / Class | Signature | Description |
|---|---|---|
| `sha256` | `(data: bytes) -> bytes` | One-shot hash, returns 32 bytes |
| `sha256_hex` | `(data: bytes) -> str` | One-shot hash, returns 64-char hex |
| `SHA256Hasher` | class | Streaming hasher |
| `.update` | `(data: bytes) -> self` | Feed bytes (chainable) |
| `.digest` | `() -> bytes` | Get 32-byte result (non-destructive) |
| `.hex_digest` | `() -> str` | Get 64-char hex result |
| `.copy` | `() -> SHA256Hasher` | Deep clone for branching |

## Testing

```bash
pytest tests/ -v
```
