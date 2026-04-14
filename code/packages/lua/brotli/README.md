# coding-adventures-brotli (Lua)

**CMP06 — Brotli lossless compression (2013)**

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) monorepo,
which builds compression algorithms from first principles.

## What Is Brotli?

Brotli is a lossless compression algorithm developed at Google (RFC 7932) that
achieves better compression than DEFLATE on web content. It is the standard
algorithm for `Content-Encoding: br` in HTTP and the WOFF2 font format.

Brotli improves on DEFLATE in three key ways:

1. **Context-dependent literal trees** — 4 separate Huffman trees, one per
   context bucket (space/punct, digit, uppercase, lowercase). Each tree is
   tuned to the statistical distribution of bytes following that type of byte.

2. **Insert-and-copy commands** — instead of separate literal and match tokens,
   one ICC (insert-copy code) symbol bundles the insert-length and copy-length
   ranges, reducing per-token overhead.

3. **Larger sliding window** — 65535 bytes (this CMP06 implementation) vs
   DEFLATE's 4096 bytes, enabling matches across much longer distances.

## Usage

```lua
local brotli = require("coding_adventures.brotli")

-- String API (most convenient)
local data = "hello hello hello world"
local compressed = brotli.compress_string(data)
local original   = brotli.decompress_string(compressed)
assert(original == data)

-- Byte-array API (for binary data)
local bytes      = {72, 101, 108, 108, 111}  -- "Hello"
local comp_bytes = brotli.compress(bytes)
local orig_bytes = brotli.decompress(comp_bytes)
```

## Wire Format

```
Header (10 bytes):
  [4B] original_length   big-endian uint32
  [1B] icc_entry_count   uint8 (1–64)
  [1B] dist_entry_count  uint8 (0–32)
  [1B] ctx0_entry_count  uint8
  [1B] ctx1_entry_count  uint8
  [1B] ctx2_entry_count  uint8
  [1B] ctx3_entry_count  uint8

ICC code-length table    (icc_entry_count × 2B):  symbol uint8, code_length uint8
Dist code-length table   (dist_entry_count × 2B): symbol uint8, code_length uint8
Literal tree 0 table     (ctx0_entry_count × 3B): symbol uint16 BE, code_length uint8
Literal tree 1 table     (ctx1_entry_count × 3B): same
Literal tree 2 table     (ctx2_entry_count × 3B): same
Literal tree 3 table     (ctx3_entry_count × 3B): same
Bit stream               LSB-first packed, zero-padded to byte boundary
```

## Context Buckets

Literals are assigned to one of 4 context buckets based on the preceding byte:

| Bucket | Preceding byte type     | ASCII ranges               |
|--------|-------------------------|----------------------------|
| 0      | Space / punctuation     | 0x00–0x2F, 0x3A–0x40, etc. |
| 1      | Digit                   | '0'–'9' (0x30–0x39)        |
| 2      | Uppercase letter        | 'A'–'Z' (0x41–0x5A)        |
| 3      | Lowercase letter        | 'a'–'z' (0x61–0x7A)        |

At the start of the stream, bucket 0 is used.

## Dependencies

- `coding-adventures-huffman-tree` (DT27) — canonical Huffman tree builder

## Running Tests

```bash
# Install dependencies first.
cd ../huffman-tree && luarocks make --local --deps-mode=none coding-adventures-huffman-tree-0.1.0-1.rockspec
cd ../brotli && luarocks make --local --deps-mode=none coding-adventures-brotli-0.1.0-1.rockspec

# Run tests.
cd tests
LUA_PATH="../src/?.lua;../src/?/init.lua;../../huffman-tree/src/?.lua;../../huffman-tree/src/?/init.lua;;" \
  busted . --verbose --pattern=test_
```

Or simply run `./BUILD` from the package root.

## Series Position

```
CMP00 (LZ77,     1977) — Sliding-window backreferences
CMP01 (LZ78,     1978) — Explicit dictionary (trie)
CMP02 (LZSS,     1982) — LZ77 + flag bits; no wasted literals
CMP03 (LZW,      1984) — LZ78 + pre-initialised alphabet; GIF
CMP04 (Huffman,  1952) — Entropy coding; prerequisite for DEFLATE
CMP05 (DEFLATE,  1996) — LZSS + dual Huffman; ZIP/gzip/PNG/zlib
CMP06 (Brotli,   2013) — Context modeling + insert-copy + large window  ← HERE
CMP07 (Zstd,     2016) — ANS/FSE + LZ4 matching; modern universal codec
```
