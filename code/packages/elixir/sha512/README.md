# coding_adventures_sha512 (Elixir)

SHA-512 cryptographic hash function (FIPS 180-4) implemented from scratch in Elixir.

## What It Does

Computes 64-byte (512-bit) digests using the SHA-2 family algorithm with 64-bit word operations. SHA-512 processes 128-byte blocks through 80 rounds of compression.

## How It Works

SHA-512 is structurally identical to SHA-256 but uses 64-bit words. Elixir's arbitrary-precision integers handle 64-bit arithmetic naturally -- we just mask with `band(x, 0xFFFFFFFFFFFFFFFF)` after additions. Binary pattern matching (`<<word::big-64, rest::binary>>`) makes parsing big-endian words elegant.

## Usage

```elixir
alias CodingAdventures.Sha512

# One-shot hashing
digest = Sha512.sha512("hello")              # 64-byte binary
hex = Sha512.sha512_hex("hello")             # 128-char hex string
```

## API

| Function | Returns | Description |
|----------|---------|-------------|
| `sha512(data)` | `binary` | 64-byte digest |
| `sha512_hex(data)` | `String.t()` | 128-char lowercase hex |

## Dependencies

None. Implemented from scratch.
