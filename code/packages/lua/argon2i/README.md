# coding-adventures-argon2i

A pure-Lua, from-scratch implementation of **Argon2i** (RFC 9106) — the
data-independent member of the Argon2 family.

## What is Argon2i?

Argon2i derives reference-block indices from a pseudo-random address
stream that depends only on the parameters, not on the password. The
memory access pattern is identical for every password, eliminating the
timing side-channel that Argon2d leaks at the cost of some GPU/ASIC
resistance. Pick this variant when strict side-channel protection
matters; otherwise prefer [`argon2id`](../argon2id/).

See the spec at [code/specs/KD03-argon2.md](../../../specs/KD03-argon2.md).

## Usage

```lua
local argon2i = require("coding_adventures.argon2i")

local tag = argon2i.argon2i("correct horse battery staple", "somesalt",
                             3, 64, 1, 32)
local hex = argon2i.argon2i_hex("password", "somesalt", 3, 64, 1, 32)
```

## API

| Function | Returns |
| -- | -- |
| `argon2i(password, salt, t, m, p, T[, opts])` | `string` (`T` bytes) |
| `argon2i_hex(password, salt, t, m, p, T[, opts])` | `string` (lowercase hex) |

## Security notes

- **Trust boundary on `memory_cost` and `tag_length`.** RFC 9106 permits
  both up to `2^32 - 1`, which translates to multi-TiB allocations.
  If either value is caller-controlled from an untrusted source, clamp
  it at the application layer.
- **Verify in constant time.** When comparing a stored tag to a freshly
  computed one, use a constant-time byte-compare rather than `==`.

## Running the tests

```bash
cd code/packages/lua/argon2i
cd tests && busted . --verbose --pattern=test_
```

Tests include the canonical RFC 9106 §5.2 gold-standard vector, plus 18
parameter-edge tests (19 total).

## Part of [coding-adventures](https://github.com/adhithyan15/coding-adventures)

One of 30 Argon2 packages across 10 languages × 3 variants (d/i/id).
