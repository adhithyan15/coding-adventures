# coding_adventures_scrypt

A pure-Rust implementation of the **scrypt** password-based key derivation
function, specified in [RFC 7914](https://www.rfc-editor.org/rfc/rfc7914) by
Colin Percival and Simon Josefsson (2016).

## What Is scrypt?

scrypt derives a cryptographic key from a password in a way that is
**deliberately expensive** for attackers. Unlike PBKDF2 (which is CPU-hard),
scrypt is **memory-hard**: it allocates a large random-access working set that
cannot be traded away for speed. An attacker running scrypt on an FPGA or GPU
still needs the same amount of RAM as a defender.

This makes brute-force attacks against stolen scrypt password hashes orders of
magnitude more expensive than attacks against PBKDF2 or bcrypt.

## Algorithm

scrypt is built from three primitives stacked together:

```
scrypt(P, S, N, r, p, dkLen):

  1. B  = PBKDF2-HMAC-SHA256(P, S, 1, p * 128 * r)
     ↑ Expand password into p independent 128*r-byte blocks

  2. For i in 0..p:
       B[i] = ROMix(B[i], N, r)
     ↑ Memory-hard mixing: requires N * 128 * r bytes of RAM

  3. DK = PBKDF2-HMAC-SHA256(P, B, 1, dkLen)
     ↑ Extract final key from memory-hard output
```

### Parameters

| Parameter | Meaning                             | Typical value |
|-----------|-------------------------------------|---------------|
| `N`       | CPU/memory cost (power of 2)        | 16384 (2^14)  |
| `r`       | Block size multiplier               | 8             |
| `p`       | Parallelisation factor              | 1             |
| `dk_len`  | Output length in bytes              | 32 or 64      |

**Memory usage**: `N * 128 * r` bytes per ROMix call.
With N=16384 and r=8: **16 MiB**.

## Usage

```rust
use coding_adventures_scrypt::{scrypt, scrypt_hex, ScryptError};

// Derive a 32-byte key
let dk = scrypt(b"my password", b"random salt", 16384, 8, 1, 32)?;
assert_eq!(dk.len(), 32);

// Or get a hex string
let hex = scrypt_hex(b"my password", b"random salt", 16384, 8, 1, 32)?;
assert_eq!(hex.len(), 64);
```

## RFC 7914 Test Vectors

This implementation passes all three RFC 7914 test vectors:

```rust
// Vector 1: empty password and salt
assert_eq!(
    scrypt_hex(b"", b"", 16, 1, 1, 64).unwrap(),
    "77d6576238657b203b19ca42c18a0497f16b4844e3074ae8dfdffa3fede21442\
     fcd0069ded0948f8326a753a0fc81f17e8d3e0fb2e0d3628cf35e20c38d18906"
);
```

Vectors 2 and 3 require significant memory (up to 16 MiB) and are run via:

```sh
cargo test -p coding_adventures_scrypt -- --ignored --nocapture
```

## Design Notes

### Internal PBKDF2

RFC 7914 vector 1 uses an empty password (`P = ""`). The published
`coding_adventures_pbkdf2` crate rejects empty passwords as a security guard.
To avoid this conflict, this crate implements its own internal
`pbkdf2_sha256_internal` that calls the low-level `hmac()` function directly,
which has no empty-key restriction.

### Salsa20/8 vs Salsa20/20

The full Salsa20 stream cipher uses 20 rounds. scrypt uses only 8 rounds
(Salsa20/8) as the mixing primitive inside BlockMix. This is safe for scrypt's
use case because Salsa20/8 is used as a pseudorandom permutation, not a stream
cipher — the security requirements are different.

## Dependencies

- `coding_adventures_hmac` — generic HMAC and HMAC-SHA256
- `coding_adventures_sha256` — SHA-256 for internal PBKDF2

## Layer in the Stack

```
scrypt (this crate)
  └── coding_adventures_hmac
        └── coding_adventures_sha256
              └── (no deps)
```

## Related Packages

- `coding_adventures_hmac` — HMAC-SHA256, used internally
- `coding_adventures_pbkdf2` — PBKDF2-HMAC-SHA256 (rejects empty passwords)
- `coding_adventures_sha256` — SHA-256 hash function
