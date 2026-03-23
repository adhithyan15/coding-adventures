# coding_adventures_sha1 (Elixir)

SHA-1 cryptographic hash function (FIPS 180-4) implemented from scratch in Elixir.

## What It Does

SHA-1 takes any sequence of bytes and produces a fixed-size 20-byte (160-bit) digest.
The same input always yields the same digest. Change one bit of input and the entire
digest changes — the avalanche effect. This package implements SHA-1 from scratch,
without using `:crypto.hash(:sha, data)`, so every step of the algorithm is visible.

## How It Fits in the Stack

This package is part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures)
monorepo. SHA-1 is a prerequisite for the UUID v5 package.

## Elixir Highlights

Elixir's binary pattern matching makes SHA-1 elegant:
- `<<word::big-32, rest::binary>>` destructures a big-endian 32-bit word.
- Guard clauses (`when t < 20`) select the auxiliary function per round.
- The pipe operator (`|>`) composes pad → process_blocks → finalize cleanly.

## Usage

```elixir
alias CodingAdventures.Sha1, as: S

# One-shot
digest = S.sha1("abc")                            # binary, 20 bytes
hex = S.sha1_hex("abc")                           # "a9993e364706816aba3e25717850c26c9cd0d89d"

# Using Base module for hex conversion
Base.encode16(S.sha1("abc"), case: :lower)        # same result
```

## FIPS 180-4 Test Vectors

```elixir
S.sha1_hex("") == "da39a3ee5e6b4b0d3255bfef95601890afd80709"
S.sha1_hex("abc") == "a9993e364706816aba3e25717850c26c9cd0d89d"
```

## Development

```bash
mix deps.get && mix test --cover
```

Tests: 22 tests, all passing.
