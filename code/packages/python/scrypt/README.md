# coding-adventures-scrypt

A pure-Python implementation of the scrypt memory-hard key derivation function
(RFC 7914). Part of the coding-adventures cryptography stack.

## What Is scrypt?

scrypt is a password-based key derivation function designed by Colin Percival
in 2009. It derives a cryptographic key from a password by:

1. **Expanding** password+salt into a large buffer using PBKDF2-SHA256
2. **Mixing** that buffer with a memory-hard function (ROMix) that requires
   `N × 128r` bytes of RAM per parallel operation
3. **Collapsing** the result back to the desired key length using PBKDF2-SHA256

The memory requirement forces attackers to allocate large amounts of RAM for
every password guess, making custom hardware attacks impractical.

## Algorithm Stack

```
scrypt
├── PBKDF2-SHA256 (internal, allows empty passwords)
│   └── HMAC-SHA256 (internal _hmac_sha256_raw)
│       └── SHA-256 (coding_adventures_sha256)
└── ROMix (RFC 7914 § 5)
    └── BlockMix (RFC 7914 § 4)
        └── Salsa20/8 core (RFC 7914 § 3)
```

## Usage

```python
from coding_adventures_scrypt import scrypt, scrypt_hex

# Derive a 32-byte key
key = scrypt(
    password=b"my secret password",
    salt=b"random 16-byte salt",  # use os.urandom(16) in production
    n=16384,   # CPU/memory cost — power of 2, >= 2
    r=8,       # block size factor
    p=1,       # parallelism factor
    dk_len=32, # output length in bytes
)

# Get the key as a hex string
key_hex = scrypt_hex(b"password", b"salt", n=16384, r=8, p=1, dk_len=32)
```

## Parameters

| Parameter | Description | Typical values |
|-----------|-------------|----------------|
| `n` | CPU/memory cost. V table = N×128r bytes. Must be a power of 2 ≥ 2. | 16384 (interactive), 1048576 (offline) |
| `r` | Block size factor. Each block = 128r bytes. | 8 |
| `p` | Parallelism. Number of independent ROMix operations. | 1–16 |
| `dk_len` | Output key length in bytes. | 32 or 64 |

## Installation

```bash
pip install coding-adventures-scrypt
```

Or from source with dependencies:

```bash
uv pip install -e ../md5 -e ../sha1 -e ../sha256 -e ../sha512 -e ../hmac -e .
```

## Testing

```bash
uv pip install -e ".[dev]"
pytest tests/ -v
```

RFC 7914 vector 2 (N=1024, r=8, p=16) is marked `@pytest.mark.slow` but not
skipped — it runs by default and takes a few seconds in pure Python.

## Where This Fits

This package is layer 6 in the cryptography stack:

```
Layer 1: md5, sha1, sha256, sha512   (hash functions)
Layer 2: hmac                        (message authentication)
Layer 3: pbkdf2                      (key derivation, PBKDF2)
Layer 4: scrypt                      (key derivation, memory-hard)  ← you are here
```

## References

- [RFC 7914](https://www.rfc-editor.org/rfc/rfc7914) — The scrypt Password-Based Key Derivation Function
- [Colin Percival's original paper](https://www.tarsnap.com/scrypt/scrypt.pdf)
