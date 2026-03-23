# md5 (Python)

MD5 message digest algorithm (RFC 1321) implemented from scratch in Python.

## What It Does

MD5 takes any sequence of bytes and produces a fixed-size 16-byte (128-bit) digest.
This package implements MD5 from scratch, without using Python's `hashlib`, so every
step of the algorithm is visible and explained.

**Security note**: MD5 is cryptographically broken (collision attacks since 2004).
Do NOT use for passwords, digital signatures, or TLS. Use for UUID v3 or legacy
checksums only.

## How It Differs From SHA-1

The most important difference is byte order:

| Property    | SHA-1      | MD5              |
|-------------|------------|------------------|
| Output size | 20 bytes   | 16 bytes         |
| State words | 5 words    | 4 words          |
| Rounds      | 80         | 64               |
| Word order  | Big-endian | **Little-endian** |

MD5 reads block words with `struct.unpack("<16I", block)` and writes the final hash
with `struct.pack("<4I", ...)` — both little-endian.

## How It Fits in the Stack

SHA-1 is a prerequisite for UUID v5; MD5 is a prerequisite for UUID v3.

## Usage

```python
from md5 import md5, md5_hex, MD5

# One-shot
digest = md5(b"abc")            # bytes, length 16
hex_str = md5_hex(b"abc")       # "900150983cd24fb0d6963f7d28e17f72"

# Streaming
h = MD5()
h.update(b"ab").update(b"c")
print(h.hexdigest())             # "900150983cd24fb0d6963f7d28e17f72"

# Copy for prefix hashing
h2 = h.copy()
```

## RFC 1321 Test Vectors

```python
assert md5_hex(b"") == "d41d8cd98f00b204e9800998ecf8427e"
assert md5_hex(b"abc") == "900150983cd24fb0d6963f7d28e17f72"
assert md5_hex(b"message digest") == "f96b697d7cb7938d525a2f31aaf161d0"
```

## Development

```bash
# On Linux/Mac
bash BUILD

# On Windows
bash BUILD_windows
```

Tests: 42 tests, 100% coverage.
