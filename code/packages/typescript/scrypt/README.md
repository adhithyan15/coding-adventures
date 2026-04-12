# @coding-adventures/scrypt

scrypt — memory-hard password-based key derivation function (RFC 7914), implemented from scratch in TypeScript.

## What It Does

scrypt derives a cryptographic key from a password and salt. It is deliberately slow and memory-intensive, making brute-force attacks using GPUs, FPGAs, and ASICs vastly more expensive than attacks against plain PBKDF2 or bcrypt.

## Where It Fits

```
Password + Salt
      │
      ▼
PBKDF2-HMAC-SHA256 (1 iteration)
      │  expands to p × 128 × r bytes
      ▼
ROMix × p                ← memory-hard step (N × 128 × r bytes scratch-pad)
      │  each block uses Salsa20/8 via BlockMix
      ▼
PBKDF2-HMAC-SHA256 (1 iteration)
      │  compresses back to dkLen bytes
      ▼
Derived Key
```

Dependencies: `@coding-adventures/hmac`, `@coding-adventures/sha256`

## Usage

```typescript
import { scrypt, scryptHex } from "@coding-adventures/scrypt";

const enc = new TextEncoder();
const password = enc.encode("correct-horse-battery-staple");
const salt = enc.encode("random-16-byte-salt");

// Derive a 32-byte key suitable for AES-256.
const key = scrypt(password, salt, 16384, 8, 1, 32);

// Or get the result as a hex string:
const hex = scryptHex(password, salt, 16384, 8, 1, 32);
```

## Parameters

| Parameter | Meaning | Typical Value |
|-----------|---------|---------------|
| `n` | CPU/memory cost (power of 2) | `16384` (interactive), `1048576` (sensitive) |
| `r` | Block size multiplier | `8` |
| `p` | Parallelism factor | `1` |
| `dkLen` | Output key length in bytes | `32` or `64` |

Memory used ≈ `N × 128 × r` bytes per call. With N=16384 and r=8 that is 16 MiB.

## RFC 7914 Test Vectors

Vector 1 (trivial):
```
password = ""  salt = ""  N=16  r=1  p=1  dkLen=64
→ 77d6576238657b203b19ca42c18a0497f16b4844e3074ae8dfdffa3fede21442
   fcd0069ded0948f8326a753a0fc81f17e8d3e0fb2e0d3628cf35e20c38d18906
```

Vector 2 (realistic):
```
password = "password"  salt = "NaCl"  N=1024  r=8  p=16  dkLen=64
→ fdbabe1c9d3472007856e7190d01e9fe7c6ad7cbc8237830e77376634b373162
   2eaf30d92e22a3886ff109279d9830dac727afb94a83ee6d8360cbdfa2cc0640
```

## Design Notes

- **Empty password accepted**: RFC 7914 vector 1 uses `password=""`. The
  `@coding-adventures/pbkdf2` package rejects empty passwords as a safety guard.
  scrypt implements PBKDF2-HMAC-SHA256 inline, calling `hmac(sha256, ...)` directly,
  so empty passwords work correctly.

- **Little-endian arithmetic**: Salsa20/8 operates on 32-bit words in little-endian
  byte order throughout. All reads use `DataView.getUint32(..., true)` and all writes
  use `DataView.setUint32(..., true)`.

- **Unsigned 32-bit arithmetic**: JavaScript bitwise ops return signed 32-bit integers.
  All intermediate values use `>>> 0` to force unsigned interpretation.

- **N limit**: N is capped at 2^20 so that `integerify`'s low-32-bit truncation is safe.
