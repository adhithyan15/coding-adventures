# coding_adventures_scrypt

Elixir implementation of the **scrypt** memory-hard password-based key derivation
function (RFC 7914).

## What Is scrypt?

scrypt derives a cryptographic key from a password and salt using large amounts of
RAM and CPU time. Unlike PBKDF2 (compute-bound only), scrypt is **memory-hard**: an
attacker cannot trade memory for speed, making GPU/ASIC attacks orders of magnitude
more expensive.

scrypt is used in Litecoin mining, macOS File Vault, Tarsnap, and many password
managers.

## Stack Position

```
scrypt (this package)
  └── PBKDF2-HMAC-SHA256 (internal, bypasses empty-key guard for RFC compliance)
        └── HMAC-SHA256
              └── SHA-256
```

The package declares `coding_adventures_pbkdf2` as a dependency but uses its own
internal PBKDF2 core to support empty passwords per RFC 7914 test vector 1.

## Algorithm (RFC 7914)

```
scrypt(password, salt, N, r, p, dk_len)
│
├─ Step 1: PBKDF2(password, salt, 1, p × 128r)
│          → p independent 128r-byte blocks
│
├─ Step 2: ROMix(block[i], N)  for each i in 0..p-1
│          Fills N-entry table, makes N pseudo-random lookups.
│          ↑ The memory-hard step: requires N × 128r bytes of RAM.
│
└─ Step 3: PBKDF2(password, mixed_blocks, 1, dk_len)
           → final key
```

The innermost primitive is **Salsa20/8**: a 64-byte permutation using 8 rounds
(4 double-rounds) of quarter-round operations on 16 uint32 words.

## Usage

```elixir
# Add to mix.exs deps:
{:coding_adventures_scrypt, path: "../scrypt"}

# Derive a 32-byte key
key = CodingAdventures.Scrypt.scrypt("my-password", "random-salt", 16384, 8, 1, 32)

# Derive a key as hex
hex = CodingAdventures.Scrypt.scrypt_hex("my-password", "random-salt", 16384, 8, 1, 32)
```

## Parameters

| Parameter | Meaning                               | Typical value      |
|-----------|---------------------------------------|--------------------|
| `n`       | CPU/memory cost (power of 2, ≥ 2)    | 16384 interactive  |
| `r`       | Block size factor                     | 8                  |
| `p`       | Parallelization factor                | 1                  |
| `dk_len`  | Output length in bytes (1–2^20)       | 32 or 64           |

Memory usage = `128 × r × N` bytes. For N=16384, r=8: **16 MiB**.

## Test Vectors

Verified against Python `hashlib.scrypt`, Go `golang.org/x/crypto/scrypt`, and OpenSSL:

```
scrypt("", "", 16, 1, 1, 64) =
  77d6576238657b203b19ca42c18a0497
  f16b4844e3074ae8dfdffa3fede21442
  fcd0069ded0948f8326a753a0fc81f17
  e8d3e0fb2e0d3628cf35e20c38d18906

scrypt("password", "NaCl", 1024, 8, 16, 64) =
  fdbabe1c9d3472007856e7190d01e9fe
  7c6ad7cbc8237830e77376634b373162
  2eaf30d92e22a3886ff109279d9830da
  c727afb94a83ee6d8360cbdfa2cc0640
```

**Note:** Some printings of RFC 7914 §12 contain typographic errors in the test vector
hex. The values above are the canonical implementation-verified outputs.

## Validation

The following inputs raise `ArgumentError`:

- `n` not a power of 2, or `n < 2`, or `n > 2^20`
- `r < 1`
- `p < 1`
- `dk_len < 1` or `dk_len > 2^20`
- `p × r > 2^30`

Empty password and empty salt are **allowed** per RFC 7914 test vector 1.

## Security Notes

- **Interactive logins**: N ≥ 16384, r = 8, p = 1 (16 MiB, ~100ms)
- **Sensitive data at rest**: N ≥ 1048576, r = 8, p = 1 (1 GiB)
- For new systems, prefer **Argon2id** (RFC 9106) — winner of the Password Hashing
  Competition with stronger theoretical properties.
- Never reuse a (password, salt) pair across different applications or purposes.

## Running Tests

```bash
mix deps.get && mix test --cover
```

Coverage: **97.56%**
