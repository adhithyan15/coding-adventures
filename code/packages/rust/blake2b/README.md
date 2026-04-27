# coding_adventures_blake2b

A from-scratch Rust implementation of the **BLAKE2b** cryptographic hash
function (RFC 7693).  No external dependencies.

## What is BLAKE2b?

BLAKE2b is a modern hash function that is:

- Faster than MD5 on 64-bit hardware.
- As secure as SHA-3 against known attacks.
- Variable output length (1..64 bytes).
- Keyed in a single pass (replaces HMAC-SHA-512).
- Parameterized with salt and personalization.

It underlies libsodium, WireGuard, Noise Protocol, IPFS content addressing,
and -- within this repo -- Argon2.

See the spec at [../../specs/HF06-blake2b.md](../../specs/HF06-blake2b.md)
for the full algorithm walkthrough.

## Usage

```rust
use coding_adventures_blake2b::{blake2b, blake2b_hex, Blake2bHasher, Blake2bOptions};

// One-shot
let digest = blake2b_hex(b"abc", &Blake2bOptions::new())?;

// Truncated digest
let short = blake2b(b"abc", &Blake2bOptions::new().digest_size(32))?;

// Keyed (MAC)
let key = b"shared secret";
let tag = blake2b(msg, &Blake2bOptions::new().key(key).digest_size(32))?;

// Streaming
let mut h = Blake2bHasher::new(&Blake2bOptions::new().digest_size(32))?;
h.update(b"partial ");
h.update(b"payload");
let out = h.hex_digest();

// Salt + personal (each exactly 16 bytes, or absent)
let salt = [0u8; 16];
let personal = [0u8; 16];
blake2b(data, &Blake2bOptions::new().salt(&salt).personal(&personal))?;
```

## Implementation notes

Rust's native `u64` has `wrapping_add` and `rotate_right`, so the source is
a clean one-to-one transliteration of the RFC -- no masking helpers, no
two-word emulation, and no `unsafe` (`#![forbid(unsafe_code)]`).

## Scope

Sequential mode only.  Tree hashing, BLAKE2s, BLAKE2bp, BLAKE2sp,
BLAKE2Xb, and BLAKE3 are intentionally out of scope -- see the
"Non-Goals" section of the spec.

## Running the tests

```bash
cargo test -p coding_adventures_blake2b
```

Tests cross-validate against fixed known-answer vectors precomputed from
Python's `hashlib.blake2b`.  The same KAT table is mirrored across every
language implementation of this package in the monorepo.
