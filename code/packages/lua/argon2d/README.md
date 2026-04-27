# coding-adventures-argon2d

A pure-Lua, from-scratch implementation of **Argon2d** (RFC 9106) — the
data-dependent member of the Argon2 family.

## What is Argon2d?

Argon2d picks the reference-block index from the first 64 bits of the
previously computed block, making memory access patterns depend on the
password. This maximises GPU/ASIC resistance but leaks a noisy side
channel through memory-access timing. Pick this variant only when
side-channel attacks are **not** in scope (e.g. proof-of-work). For
password hashing, prefer [`argon2id`](../argon2id/).

See the spec at [code/specs/KD03-argon2.md](../../../specs/KD03-argon2.md).

## Usage

```lua
local argon2d = require("coding_adventures.argon2d")

local tag = argon2d.argon2d("correct horse battery staple", "somesalt",
                             3, 64, 1, 32)
local hex = argon2d.argon2d_hex("password", "somesalt", 3, 64, 1, 32)
```

### Keyed / authenticated data

```lua
argon2d.argon2d(password, salt, 3, 64, 1, 32, {
    key = server_secret,
    associated_data = "user:alice",
})
```

## API

| Function | Returns |
| -- | -- |
| `argon2d(password, salt, t, m, p, T[, opts])` | `string` (`T` bytes) |
| `argon2d_hex(password, salt, t, m, p, T[, opts])` | `string` (lowercase hex) |

Parameters follow RFC 9106 §3.1: `t` = time cost (passes), `m` = memory
in KiB, `p` = parallelism (lanes), `T` = tag length in bytes.

## Security notes

- **Trust boundary on `memory_cost` and `tag_length`.** RFC 9106 permits
  both up to `2^32 - 1`, which translates to multi-TiB allocations.
  If either value is caller-controlled from an untrusted source, clamp
  it at the application layer.
- **Verify in constant time.** When comparing a stored tag to a freshly
  computed one, use a constant-time byte-compare (e.g. the classic
  XOR-accumulate over both strings) rather than `==`.

## Running the tests

```bash
cd code/packages/lua/argon2d
cd tests && busted . --verbose --pattern=test_
```

Tests include the canonical RFC 9106 §5.1 gold-standard vector, plus 18
parameter-edge tests (19 total).

## Part of [coding-adventures](https://github.com/adhithyan15/coding-adventures)

One of 30 Argon2 packages across 10 languages × 3 variants (d/i/id).
