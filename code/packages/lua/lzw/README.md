# coding-adventures-lzw

LZW (Lempel-Ziv-Welch, 1984) lossless compression — CMP03.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) monorepo compression series.

## What Is LZW?

LZW is LZ78 with a pre-seeded dictionary: all 256 single-byte sequences are added before encoding begins (codes 0–255). This eliminates LZ78's mandatory `next_char` byte that followed every token, because every possible byte is already in the dictionary from the start.

With only codes to transmit, LZW uses variable-width bit-packing: codes start at 9 bits and grow as the dictionary expands. This is exactly how GIF compression works.

## Reserved Codes

| Code | Meaning |
|------|---------|
| 0–255 | Pre-seeded single-byte entries |
| 256 | `CLEAR_CODE` — reset dictionary and code size |
| 257 | `STOP_CODE` — end of code stream |
| 258+ | Dynamically assigned sequences |

## Wire Format (CMP03)

```
Bytes 0–3:  original_length  (big-endian uint32)
Bytes 4+:   bit-packed variable-width codes, LSB-first
```

No block-count header is needed — the `STOP_CODE` terminates the stream.

## Usage

```lua
local lzw = require("coding_adventures.lzw")

-- Compress
local compressed = lzw.compress("ABABABABABABABABAB")
print(#compressed, "bytes compressed")

-- Decompress
local original = lzw.decompress(compressed)
print(original)  -- → "ABABABABABABABABAB"
```

## API

### `lzw.compress(str) → string`

Compresses a Lua string using LZW and returns a CMP03 binary string.

### `lzw.decompress(data) → string`

Decompresses CMP03 wire-format bytes back to the original string.

## Constants

```lua
lzw.CLEAR_CODE        -- 256
lzw.STOP_CODE         -- 257
lzw.INITIAL_NEXT_CODE -- 258
lzw.INITIAL_CODE_SIZE -- 9
lzw.MAX_CODE_SIZE     -- 16
lzw.VERSION           -- "0.1.0"
```

## The Compression Series

| ID | Algorithm | Year | Description |
|----|-----------|------|-------------|
| CMP00 | LZ77 | 1977 | Sliding-window backreferences |
| CMP01 | LZ78 | 1978 | Explicit dictionary (trie) |
| CMP02 | LZSS | 1982 | LZ77 + flag bits; no wasted literals |
| **CMP03** | **LZW** | **1984** | **Pre-initialised dictionary; powers GIF** |
| CMP04 | Huffman | 1952 | Entropy coding |
| CMP05 | DEFLATE | 1996 | LZ77 + Huffman; ZIP/gzip/PNG/zlib |

## Running Tests

```bash
cd tests && LUA_PATH="../src/?.lua;../src/?/init.lua;;" busted . --verbose --pattern=test_
```

## Requirements

- Lua >= 5.4 (uses native `|`, `&`, `<<`, `>>` bit operators)
- [Busted](https://lunarmodules.github.io/busted/) for running tests
