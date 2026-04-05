# coding-adventures-sha256 (Lua)

Pure Lua implementation of the SHA-256 cryptographic hash function (FIPS 180-4).

## Overview

SHA-256 is a member of the SHA-2 family that produces a 256-bit (32-byte) digest. It uses the Merkle-Damgard construction with 8 x 32-bit state words and 64 compression rounds per block. Unlike MD5 and SHA-1, SHA-256 has no known practical attacks.

This implementation requires Lua 5.4+ for native 64-bit integers and bitwise operators.

## Installation

```bash
luarocks install coding-adventures-sha256
```

## Usage

### One-shot API

```lua
local sha256 = require("coding_adventures.sha256")

-- Hex digest (64-character lowercase string)
local hex = sha256.sha256_hex("hello")
-- "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"

-- Raw digest (table of 32 integers, each 0-255)
local raw = sha256.sha256("hello")
```

### Streaming API

```lua
local sha256 = require("coding_adventures.sha256")

local hasher = sha256.new()
hasher:update("hello ")
hasher:update("world")
print(hasher:hex_digest())  -- same as sha256.sha256_hex("hello world")

-- Non-destructive: can call digest multiple times
local d1 = hasher:hex_digest()
local d2 = hasher:hex_digest()
assert(d1 == d2)

-- Copy for branching
local branch = hasher:copy()
branch:update("!")
-- branch and hasher now have different states
```

## API Reference

| Function | Returns | Description |
|----------|---------|-------------|
| `sha256(data)` | table of 32 ints | Raw 32-byte digest |
| `sha256_hex(data)` | string | 64-char lowercase hex digest |
| `new()` | Hasher | Create streaming hasher |
| `hasher:update(data)` | self | Feed bytes (chainable) |
| `hasher:digest()` | table of 32 ints | Get digest (non-destructive) |
| `hasher:hex_digest()` | string | Get hex digest (non-destructive) |
| `hasher:copy()` | Hasher | Deep copy for branching |

## Part of coding-adventures

An educational computing stack built from logic gates up through interpreters and compilers. This package implements HF03 (SHA-256) from the hash functions layer.
