# coding-adventures-argon2id

A pure-Lua, from-scratch implementation of **Argon2id** (RFC 9106) —
the RFC-recommended memory-hard password hashing function.

## What is Argon2id?

Argon2id is the "hybrid" member of the Argon2 family: the first half
of the first pass uses side-channel-resistant **data-independent**
addressing (Argon2i), and everything afterwards uses GPU/ASIC-resistant
**data-dependent** addressing (Argon2d). Pick this variant unless you
have a specific reason to prefer [`argon2d`](../argon2d/) (proof-of-work,
no side-channel threat) or [`argon2i`](../argon2i/) (strict side-channel
requirements).

See the spec at [code/specs/KD03-argon2.md](../../../specs/KD03-argon2.md).

## Usage

```lua
local argon2id = require("coding_adventures.argon2id")

local tag = argon2id.argon2id("correct horse battery staple", "somesalt",
                                3, 64, 1, 32)
local hex = argon2id.argon2id_hex("password", "somesalt", 3, 64, 1, 32)
```

### Keyed / authenticated data

```lua
argon2id.argon2id(password, salt, 3, 64, 1, 32, {
    key = server_secret,
    associated_data = "user:alice",
})
```

## API

| Function | Returns |
| -- | -- |
| `argon2id(password, salt, t, m, p, T[, opts])` | `string` (`T` bytes) |
| `argon2id_hex(password, salt, t, m, p, T[, opts])` | `string` (lowercase hex) |

Parameters follow RFC 9106 §3.1: `t` = time cost (passes), `m` = memory
in KiB, `p` = parallelism (lanes), `T` = tag length in bytes.

## Security notes

- **Trust boundary on `memory_cost` and `tag_length`.** RFC 9106 permits
  both up to `2^32 - 1`, which translates to multi-TiB allocations. If
  either value is caller-controlled from an untrusted source, clamp it
  at the application layer.
- **Verify in constant time.** When comparing a stored tag to a freshly
  computed one, use a constant-time byte-compare rather than `==`.

## Running the tests

```bash
cd code/packages/lua/argon2id
cd tests && busted . --verbose --pattern=test_
```

Tests include the canonical RFC 9106 §5.3 gold-standard vector, plus 18
parameter-edge tests (19 total).

## Part of [coding-adventures](https://github.com/adhithyan15/coding-adventures)

One of 30 Argon2 packages across 10 languages × 3 variants (d/i/id).
