# coding_adventures_scrypt

scrypt key derivation function (RFC 7914), implemented from scratch in Ruby.

## What Is scrypt?

scrypt is a memory-hard password-based key derivation function (PBKDF) designed
by Colin Percival (2009). It uses deliberately large amounts of memory to make
brute-force attacks expensive even with specialised hardware (GPUs, FPGAs, ASICs).

The core idea: deriving a single candidate password costs O(N) memory and O(N)
time. With N=16384 and r=8, each attempt requires ~16 MiB of RAM — limiting
an attacker to a handful of guesses per second per gigabyte of hardware.

## Where Does scrypt Fit in the Stack?

```
coding_adventures_scrypt
    └── coding_adventures_hmac      (HMAC-SHA256 via inline PBKDF2)
            └── coding_adventures_sha256
```

scrypt is built on HMAC-SHA256 (via an inline PBKDF2-HMAC-SHA256 that supports
empty passwords, as required by RFC 7914 vector 1). The memory-hard mixing is
performed by repeated applications of Salsa20/8 (BlockMix) within RoMix.

## Usage

```ruby
require "coding_adventures_scrypt"

# Derive a 32-byte key for interactive login (N=16384, r=8, p=1 ≈ 16 MiB, ~0.5 s)
key = CodingAdventures::Scrypt.scrypt("hunter2", "random_salt_bytes", 16384, 8, 1, 32)

# Hex string variant — useful for logging and comparison with published vectors
hex = CodingAdventures::Scrypt.scrypt_hex("hunter2", "random_salt_bytes", 16384, 8, 1, 32)

# RFC 7914 vector 1 (empty password — verified against OpenSSL and Python)
CodingAdventures::Scrypt.scrypt_hex("", "", 16, 1, 1, 64)
# => "77d6576238657b203b19ca42c18a0497f16b4844..."
```

## Parameters

| Parameter | Meaning | Recommended |
|-----------|---------|-------------|
| `n`       | CPU/memory cost — must be power of 2 | 16384 (interactive), 1048576 (sensitive) |
| `r`       | Block size factor | 8 |
| `p`       | Parallelisation factor | 1 |
| `dk_len`  | Output length in bytes | 32 or 64 |

Memory used ≈ 128 × n × r bytes per lane. With n=16384, r=8: ~16 MiB.

## RFC 7914 Test Vectors

```ruby
# Vector 1 (verified against OpenSSL and Python hashlib.scrypt)
CodingAdventures::Scrypt.scrypt_hex("", "", 16, 1, 1, 64)
# => "77d6576238657b203b19ca42c18a0497f16b4844e3074ae8dfdffa3fede21442
#     fcd0069ded0948f8326a753a0fc81f17e8d3e0fb2e0d3628cf35e20c38d18906"

# Vector 2 (verified against OpenSSL and Python hashlib.scrypt)
CodingAdventures::Scrypt.scrypt_hex("password", "NaCl", 1024, 8, 16, 64)
# => "fdbabe1c9d3472007856e7190d01e9fe7c6ad7cbc8237830e77376634b373162
#     2eaf30d92e22a3886ff109279d9830dac727afb94a83ee6d8360cbdfa2cc0640"
```

## Running Tests

```bash
bundle install
bundle exec rake test
```

## Architecture

The implementation follows the RFC 7914 structure exactly:

1. **PBKDF2-HMAC-SHA256** (inline) — expands `(password, salt)` into `p × 128r` bytes.
2. **RoMix** (per lane) — fills an N-entry lookup table, then makes N pseudo-random
   lookups into that table using **BlockMix** and **Salsa20/8**.
3. **PBKDF2-HMAC-SHA256** (inline) — compresses the scrambled lanes into `dk_len` bytes.

All code is written in Knuth literate-programming style — the source file
explains every function, data structure, and design decision at length, suitable
for a reader encountering cryptographic primitives for the first time.
