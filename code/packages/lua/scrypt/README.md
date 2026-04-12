# coding-adventures-scrypt

A pure-Lua 5.4 implementation of the **scrypt** key derivation function
(RFC 7914), written from scratch for the coding-adventures monorepo.

## What Is scrypt?

scrypt is a **memory-hard** password hashing and key derivation function
designed by Colin Percival in 2009. Unlike PBKDF2 (which is purely CPU-hard),
scrypt requires a configurable amount of RAM per derivation. This forces
attackers with specialised hardware (ASICs, FPGAs, GPUs) to either buy lots of
RAM (expensive) or recompute O(N²) work per guess (also expensive).

Real-world uses: Litecoin proof-of-work, 1Password, libsodium, many web frameworks.

## Module

```
coding_adventures.scrypt
```

## API

### `scrypt(password, salt, N, r, p, dk_len) → string`

Derive a key. Returns a **raw binary string** of `dk_len` bytes.

| Parameter | Type    | Description |
|-----------|---------|-------------|
| password  | string  | Secret passphrase. May be empty (RFC 7914 vector 1 uses `""`). |
| salt      | string  | Unique random salt per credential. |
| N         | integer | CPU/memory cost. Must be a power of 2 ≥ 2, ≤ 2²⁰. |
| r         | integer | Block size multiplier ≥ 1. |
| p         | integer | Parallelisation factor ≥ 1. |
| dk_len    | integer | Output length in bytes (1 .. 2²⁰). |

### `scrypt_hex(password, salt, N, r, p, dk_len) → string`

Like `scrypt` but returns a lowercase hexadecimal string (2 × dk_len chars).

## Parameters Guide

| Use case | N | r | p |
|----------|---|---|---|
| Interactive login (2023) | 16384 | 8 | 1 |
| High-security offline | 1048576 | 8 | 1 |
| RFC 7914 vector 1 | 16 | 1 | 1 |
| RFC 7914 vector 2 | 1024 | 8 | 16 |

## Examples

```lua
local scrypt = require("coding_adventures.scrypt")

-- Derive a 32-byte key for interactive login
local key_hex = scrypt.scrypt_hex("my password", "random salt", 16384, 8, 1, 32)
-- key_hex is a 64-character lowercase hex string

-- RFC 7914 vector 1 (empty password and salt)
local v1 = scrypt.scrypt_hex("", "", 16, 1, 1, 64)
-- "77d6576238657b203b19ca42c18a0497..."

-- RFC 7914 vector 2
local v2 = scrypt.scrypt_hex("password", "NaCl", 1024, 8, 16, 64)
-- "fdbabe1c9d3472007856e7190d01e72c..."
```

## Algorithm Overview

```
scrypt(P, S, N, r, p, dkLen):
  B  = PBKDF2-HMAC-SHA256(P, S, 1, p × 128r)      ← expand
  for i = 0..p-1:
    B[i] = ROMix(B[i], N, r)                        ← memory-hard mix
  DK = PBKDF2-HMAC-SHA256(P, B, 1, dkLen)           ← extract
  return DK

ROMix(B, N, r):
  V[1..N] = sequential snapshots of BlockMix(B)     ← fill RAM table
  for i = 1..N:
    j = Integerify(B) mod N + 1
    B = BlockMix(B XOR V[j])                        ← pseudo-random lookup
  return B
```

## Dependencies

- `coding-adventures-hmac` (for HMAC-SHA256)
- `coding-adventures-sha256` (SHA-256 primitive)

## Running Tests

```bash
cd tests && busted . --verbose --pattern=test_
```

## Test Vectors

RFC 7914 §11 vectors 1 and 2 are both verified in the test suite.

## Stack Position

```
scrypt (this package)
  └── coding-adventures-hmac
        ├── coding-adventures-sha256
        ├── coding-adventures-sha512
        ├── coding-adventures-sha1
        └── coding-adventures-md5
```

## License

MIT
