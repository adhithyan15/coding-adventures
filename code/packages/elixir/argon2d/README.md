# coding_adventures_argon2d

A pure-Elixir, from-scratch implementation of **Argon2d** (RFC 9106) —
data-dependent memory-hard password hashing.

## What is Argon2d?

Argon2d picks the reference-block index for every new block from the
first 64 bits of the previously computed block. The memory-access
pattern therefore depends on the password, which maximises GPU/ASIC
resistance at the cost of leaking a noisy channel through memory-access
timing. Use Argon2d only when side-channel attacks are *not* in the
threat model (e.g. proof-of-work). For password hashing prefer
[`argon2id`](../argon2id/).

See the spec at [code/specs/KD03-argon2.md](../../../specs/KD03-argon2.md).

## Usage

```elixir
tag = CodingAdventures.Argon2d.argon2d("password", "somesalt", 3, 64, 1, 32)
hex = CodingAdventures.Argon2d.argon2d_hex("password", "somesalt", 3, 64, 1, 32)
```

### Keyed / authenticated data

```elixir
CodingAdventures.Argon2d.argon2d(
  password, salt, 3, 64, 1, 32,
  key: secret,
  associated_data: "challenge-id"
)
```

## API

| Function | Returns |
| -- | -- |
| `argon2d(password, salt, t, m, p, T, opts \\ [])` | `binary` (`T` bytes) |
| `argon2d_hex(password, salt, t, m, p, T, opts \\ [])` | `String.t()` (lowercase hex) |

Parameters follow RFC 9106 §3.1.

## Where this fits in the stack

- **Dependencies:** [`coding_adventures_blake2b`](../blake2b/) (H0 and H' extender).

## Security notes

- **Trust boundary on `memory_cost` and `tag_length`.** RFC 9106 permits
  both up to `2^32 - 1`, which translates to multi-TiB allocations. Clamp
  caller-controlled values at the application layer.
- **Verify in constant time.** Use `Plug.Crypto.secure_compare/2` or
  `:crypto.hash_equals/2` (OTP 25+) rather than `==` when comparing a
  stored tag against a freshly computed one.

## Running the tests

```bash
cd code/packages/elixir/argon2d
mix deps.get && mix test --cover
```

Tests include the canonical RFC 9106 §5.1 gold-standard vector, plus
18 parameter-edge tests (19 total).

## Part of [coding-adventures](https://github.com/adhithyan15/coding-adventures)

One of 30 Argon2 packages across 10 languages × 3 variants (d/i/id).
