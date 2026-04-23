# BLAKE2b (Lua)

Pure Lua implementation of the **BLAKE2b** cryptographic hash function
(RFC 7693).

See the spec at [../../specs/HF06-blake2b.md](../../specs/HF06-blake2b.md)
for the full walk-through.

## Requirements

- Lua 5.3+ (native 64-bit integers and bitwise operators)

## Usage

```lua
local blake2b = require("coding_adventures.blake2b")

-- One-shot
local hex = blake2b.hex("hello")                    -- 128-char hex string
local raw = blake2b.digest("hello")                 -- 64-byte raw string

-- Variable digest size
local tag = blake2b.digest("hello", { digest_size = 32 })   -- 32-byte

-- Keyed (MAC mode)
local mac = blake2b.hex(message, { key = "shared-secret" })

-- Streaming
local h = blake2b.Hasher.new({ digest_size = 32 })
h:update("hello ")
h:update("world")
local out = h:hex_digest()

-- Salt + personal (each exactly 16 bytes, or absent)
local salt     = string.rep("\0", 16)
local personal = string.rep("\0", 16)
local dsep = blake2b.hex(data, { salt = salt, personal = personal })
```

## API

| Function | Returns | Description |
|----------|---------|-------------|
| `blake2b.digest(msg, opts?)` | string | Raw digest bytes |
| `blake2b.hex(msg, opts?)` | string | Lowercase hex digest |
| `blake2b.Hasher.new(opts?)` | hasher | Streaming hasher value |
| `hasher:update(data)` | hasher | Absorb more bytes |
| `hasher:digest()` | string | Finalize (non-destructive) |
| `hasher:hex_digest()` | string | Finalize to lowercase hex |
| `hasher:copy()` | hasher | Independent deep copy |

`opts` may contain any of `digest_size` (1..64, default 64), `key` (0..64 bytes),
`salt` (exactly 0 or 16 bytes), `personal` (exactly 0 or 16 bytes).

## Implementation notes

Lua 5.3+ has native 64-bit integers with wrap-on-overflow addition and a
*logical* `>>` right shift, so the RFC's `(x + y) mod 2^64` and
`ROTR(x, n) = (x >> n) | (x << (64 - n))` translate directly without
any 64-bit emulation.  This file has no C extensions.

The 128-bit byte counter is represented as a single 64-bit Lua integer,
which is sufficient for any practical input (2^64 - 1 bytes = 16 EB).
The spec's reserved high 64 bits are zero for every realistic message.

`Hasher` is a metatable-backed value type.  `copy()` returns an
independent hasher with a fresh state table and buffer.

## Scope

Sequential mode only.  Tree hashing, BLAKE2s, BLAKE2bp, BLAKE2sp,
BLAKE2Xb, and BLAKE3 are out of scope per the HF06 spec.

## Running the tests

```bash
cd tests && busted . --verbose --pattern=test_
```

Tests cross-validate against fixed known-answer vectors precomputed from
Python's `hashlib.blake2b`.  The same KAT table is mirrored across every
language implementation in the monorepo.

## Part of coding-adventures

An educational computing stack built from logic gates up through
interpreters and compilers.  BLAKE2b is a prerequisite for Argon2
(the memory-hard password hashing function).
