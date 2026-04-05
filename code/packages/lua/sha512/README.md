# SHA-512 (Lua)

Pure Lua implementation of the SHA-512 cryptographic hash function (FIPS 180-4).

## Overview

SHA-512 is the 64-bit sibling of SHA-256 in the SHA-2 family. It produces a 512-bit (64-byte) digest using 8 x 64-bit state words and 80 rounds of compression. On 64-bit platforms, SHA-512 is often faster than SHA-256 because it processes 128-byte blocks using native 64-bit arithmetic.

## Requirements

- Lua 5.3+ (requires native 64-bit integers and bitwise operators)

## Usage

```lua
local sha512 = require("coding_adventures.sha512")

-- Hex digest (128-character lowercase string)
local hex = sha512.hex("hello")

-- Raw digest (table of 64 integers, each 0-255)
local raw = sha512.digest("hello")
```

## API

| Function | Returns | Description |
|----------|---------|-------------|
| `sha512.hex(message)` | string | 128-character lowercase hex digest |
| `sha512.digest(message)` | table | 64-element table of byte values (0-255) |

## Part of coding-adventures

An educational computing stack built from logic gates up through interpreters and compilers.
