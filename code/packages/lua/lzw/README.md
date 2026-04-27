# coding-adventures-lzw (Lua)

LZW (Lempel-Ziv-Welch, 1984) lossless compression — CMP03 in the coding-adventures series.

## Usage

```lua
local lzw = require("coding_adventures.lzw")

local data       = "TOBEORNOTTOBEORTOBEORNOT"
local compressed = lzw.compress(data)
local original   = lzw.decompress(compressed)
```

## Wire Format (CMP03)

```
Bytes 0-3:  original_length (big-endian uint32)
Bytes 4+:   variable-width codes, LSB-first bit-packed
              - code_size starts at 9 bits, grows as dict expands
              - first code: CLEAR_CODE (256)
              - last code:  STOP_CODE  (257)
```

## Series

```
CMP00 (LZ77,    1977) — Sliding-window backreferences
CMP01 (LZ78,    1978) — Explicit dictionary (trie)
CMP02 (LZSS,    1982) — LZ77 + flag bits
CMP03 (LZW,     1984) — LZ78 + pre-initialized dict; powers GIF  ← this package
CMP04 (Huffman, 1952) — Entropy coding
CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib
```
