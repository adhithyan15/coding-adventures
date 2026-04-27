# coding_adventures_argon2i

A pure-Elixir, from-scratch implementation of **Argon2i** (RFC 9106) —
data-independent memory-hard password hashing.

## What is Argon2i?

Argon2i derives reference-block indices from a deterministic
pseudo-random stream seeded purely from public parameters (pass,
lane, slice, counter, total memory, total passes, type). Memory
access patterns leak nothing about the password, which defeats
side-channel observers at the cost of making the variant the easiest
for GPUs/ASICs to parallelise. For general password hashing prefer
[`argon2id`](../argon2id/); use Argon2i only when side-channel
resistance is the dominant concern.

See the spec at [code/specs/KD03-argon2.md](../../../specs/KD03-argon2.md).

## Usage

```elixir
tag = CodingAdventures.Argon2i.argon2i("password", "somesalt", 3, 64, 1, 32)
hex = CodingAdventures.Argon2i.argon2i_hex("password", "somesalt", 3, 64, 1, 32)
```

## API

| Function | Returns |
| -- | -- |
| `argon2i(password, salt, t, m, p, T, opts \\ [])` | `binary` (`T` bytes) |
| `argon2i_hex(password, salt, t, m, p, T, opts \\ [])` | `String.t()` (lowercase hex) |

Parameters follow RFC 9106 §3.1.

## Security notes

- **Trust boundary on `memory_cost` and `tag_length`.** RFC 9106 permits
  both up to `2^32 - 1`, which translates to multi-TiB allocations.
- **Verify in constant time.** Use `Plug.Crypto.secure_compare/2` or
  `:crypto.hash_equals/2` (OTP 25+) rather than `==`.

## Running the tests

```bash
cd code/packages/elixir/argon2i
mix deps.get && mix test --cover
```

Tests include the canonical RFC 9106 §5.2 gold-standard vector, plus
18 parameter-edge tests (19 total).

## Part of [coding-adventures](https://github.com/adhithyan15/coding-adventures)

One of 30 Argon2 packages across 10 languages × 3 variants (d/i/id).
