# coding_adventures_blake2b

A from-scratch Ruby implementation of the **BLAKE2b** cryptographic hash
function (RFC 7693).  No external runtime dependencies.

## What is BLAKE2b?

BLAKE2b is a modern hash that is:

- Faster than MD5 on 64-bit hardware.
- As secure as SHA-3 against known attacks.
- Variable output length (1..64 bytes).
- Keyed in a single pass (replaces HMAC-SHA-512).
- Parameterized with salt and personalization.

It underlies libsodium, WireGuard, Noise Protocol, IPFS content addressing,
and -- within this repo -- Argon2.

See the spec at [../../specs/HF06-blake2b.md](../../specs/HF06-blake2b.md)
for the full walk-through.

## Usage

```ruby
require "coding_adventures_blake2b"

Blake2b = CodingAdventures::Blake2b

# One-shot
Blake2b.blake2b_hex("abc".b)                         # 128-char hex
Blake2b.blake2b("abc".b, digest_size: 32)            # 32 raw bytes

# Keyed (MAC)
Blake2b.blake2b(message, key: "shared secret".b, digest_size: 32)

# Streaming
h = Blake2b::Hasher.new(digest_size: 32)
h.update("partial ".b)
h.update("payload".b)
h.hex_digest

# Salt + personal (each exactly 16 bytes, or absent)
Blake2b.blake2b(data, salt: "a" * 16, personal: "b" * 16)
```

## Implementation notes

Ruby has arbitrary-precision integers, so every 64-bit add and XOR in the
compression function masks with `MASK64 = 0xFFFFFFFFFFFFFFFF` to prevent
the result from silently growing into a Bignum.  That is the main
structural difference from the Go and Rust ports where wrapping `u64`
arithmetic is native.

## Scope

Sequential mode only.  Tree hashing, BLAKE2s, BLAKE2bp, BLAKE2sp,
BLAKE2Xb, and BLAKE3 are intentionally out of scope -- see the
"Non-Goals" section of the spec.

## Running the tests

```bash
bundle install
bundle exec rake test
```

Tests cross-validate against fixed known-answer vectors precomputed from
Python's `hashlib.blake2b`.  The same KAT table is mirrored across every
language implementation in the monorepo.
