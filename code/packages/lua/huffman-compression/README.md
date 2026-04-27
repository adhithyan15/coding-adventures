# coding-adventures-huffman-compression

**CMP04 — Huffman Lossless Compression (1952)**

A pure-Lua implementation of Huffman compression and decompression, implementing
the CMP04 wire format from the coding-adventures specification.

## What It Does

Huffman compression assigns shorter bit codes to more-frequent byte values and
longer codes to less-frequent values, producing an entropy-optimal prefix-free
encoding. It is a prerequisite for DEFLATE (used in ZIP, gzip, PNG, and zlib).

This package builds on **DT27 HuffmanTree** (`coding-adventures-huffman-tree`)
for tree construction and canonical code generation.

## CMP04 Wire Format

```
Bytes 0–3:    original_length  (big-endian uint32)
Bytes 4–7:    symbol_count     (big-endian uint32)
Bytes 8–8+2N: code-lengths table — N entries × 2 bytes:
                [0] symbol value (uint8)
                [1] code length  (uint8)
              Sorted by (code_length, symbol_value) ascending.
Bytes 8+2N+:  bit stream — packed LSB-first, zero-padded to byte boundary.
```

## Usage

```lua
local hc = require("coding_adventures.huffman_compression")

local compressed   = hc.compress("AAABBC")
local decompressed = hc.decompress(compressed)
-- decompressed == "AAABBC"
```

## Installation

```bash
luarocks make --local coding-adventures-huffman-compression-0.1.0-1.rockspec
```

Requires `coding-adventures-huffman-tree` (DT27) to be installed first:

```bash
cd ../huffman-tree
luarocks make --local coding-adventures-huffman-tree-0.1.0-1.rockspec
```

## Running Tests

```bash
cd tests
LUA_PATH="../src/?.lua;../src/?/init.lua;../../huffman-tree/src/?.lua;../../huffman-tree/src/?/init.lua;;" \
  busted . --verbose --pattern=test_
```

## Series

| ID    | Algorithm | Year | Notes                              |
|-------|-----------|------|------------------------------------|
| CMP00 | LZ77      | 1977 | Sliding-window backreferences      |
| CMP01 | LZ78      | 1978 | Explicit dictionary (trie)         |
| CMP02 | LZSS      | 1982 | LZ77 + flag bits                   |
| CMP03 | LZW       | 1984 | Pre-initialised dictionary (GIF)   |
| CMP04 | Huffman   | 1952 | Entropy coding — **this package**  |
| CMP05 | DEFLATE   | 1996 | LZ77 + Huffman (ZIP/gzip/PNG)      |

## License

MIT
