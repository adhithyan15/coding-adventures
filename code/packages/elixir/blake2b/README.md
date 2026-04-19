# coding_adventures_blake2b (Elixir)

A from-scratch Elixir implementation of the **BLAKE2b** cryptographic
hash function (RFC 7693).  No external runtime dependencies.

See the spec at [../../specs/HF06-blake2b.md](../../specs/HF06-blake2b.md)
for the full walk-through.

## Usage

```elixir
alias CodingAdventures.Blake2b

# One-shot
Blake2b.blake2b_hex("abc")                              # 128-char hex
Blake2b.blake2b("abc", digest_size: 32)                 # 32 raw bytes

# Keyed (MAC)
Blake2b.blake2b(msg, key: "shared secret", digest_size: 32)

# Streaming
h =
  Blake2b.new(digest_size: 32)
  |> Blake2b.update("partial ")
  |> Blake2b.update("payload")

Blake2b.hex_digest(h)

# Salt + personal (each exactly 16 bytes, or absent)
Blake2b.blake2b(data,
  salt:     :binary.copy(<<0>>, 16),
  personal: :binary.copy(<<0>>, 16)
)
```

## Implementation notes

Elixir integers are arbitrary precision, so every 64-bit add and XOR
masks with `0xFFFFFFFFFFFFFFFF` (via `&&&`) to stay inside a single
machine word.  That is the main difference from the Go and Rust ports,
which use native wrapping `u64` arithmetic.

The hasher is a plain immutable struct threaded through
`update/2` / `digest/1` pipelines -- Elixir-idiomatic, and naturally
non-destructive.

## Scope

Sequential mode only.  Tree hashing, BLAKE2s, BLAKE2bp, BLAKE2sp,
BLAKE2Xb, and BLAKE3 are intentionally out of scope.

## Running the tests

```bash
mix test --cover
```

Tests cross-validate against fixed known-answer vectors precomputed from
Python's `hashlib.blake2b`.  The same KAT table is mirrored across every
language implementation of this package in the monorepo.
