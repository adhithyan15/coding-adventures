# coding-adventures-zstd

ZStd (RFC 8878) lossless compression implemented from scratch in Lua 5.4 — **CMP07** in the compression series.

## What it does

Compresses arbitrary binary data to the ZStd frame format (RFC 8878) and decompresses it back. The output is a byte-compatible ZStd frame that can also be decompressed by the `zstd` command-line tool or any conforming ZStd implementation.

## Where it fits

```
CMP00 (LZ77,    1977) — Sliding-window back-references
CMP02 (LZSS,    1982) — LZ77 + flag bits              ← dependency
CMP04 (Huffman, 1952) — Entropy coding
CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG
CMP06 (Brotli,  2013) — DEFLATE + context modelling
CMP07 (ZStd,    2015) — LZ77 + FSE; high ratio + speed ← this package
```

ZStd combines two ideas: **LZ77 back-references** (via the LZSS sub-package) to exploit repetition, and **FSE (Finite State Entropy)** coding for sequence descriptors. FSE is an ANS-variant that approaches the Shannon entropy limit in a single pass, outperforming Huffman on most real data.

## Dependencies

- `coding-adventures-lzss` — LZ77/LZSS token generator (must be installed first)

## Installation

```bash
# Install the LZSS dependency first
cd ../lzss && luarocks make --local --deps-mode=none coding-adventures-lzss-0.1.0-1.rockspec

# Install this package
luarocks make --local --deps-mode=none coding-adventures-zstd-0.1.0-1.rockspec
```

## Usage

```lua
local zstd = require("coding_adventures.zstd")

-- Compress a string
local compressed = zstd.compress("the quick brown fox jumps over the lazy dog")

-- Decompress back to the original
local original = zstd.decompress(compressed)
assert(original == "the quick brown fox jumps over the lazy dog")

-- Both functions accept strings or byte arrays (tables of integers 0–255)
-- and always return a Lua string.

-- Highly repetitive data compresses extremely well
local rle_input = string.rep("\xAB", 200 * 1024)   -- 200 KB
local rle_out   = zstd.compress(rle_input)
print(#rle_out)  -- ~21 bytes (two RLE blocks + frame header)
```

## API

| Function | Input | Output | Description |
|----------|-------|--------|-------------|
| `compress(data)` | `string` or `{int…}` | `string` | Compress to a ZStd frame. |
| `decompress(data)` | `string` or `{int…}` | `string` | Decompress a ZStd frame. |

`decompress` calls `error()` if the frame is malformed, truncated, uses an unsupported feature (non-predefined FSE tables, Huffman literals), or if the decompressed output exceeds 256 MB (bomb guard).

## Algorithm overview

### Frame layout (RFC 8878 §3)

```
┌────────┬─────┬──────────────────────┬────────┐
│ Magic  │ FHD │ Frame_Content_Size   │ Blocks │
│ 4 B LE │ 1 B │ 8 B LE               │ ...    │
└────────┴─────┴──────────────────────┴────────┘
```

Magic bytes: `0x28 0xB5 0x2F 0xFD` (= 0xFD2FB528 LE).

### Block types

Each block has a 3-byte little-endian header:

| Bits | Field | Values |
|------|-------|--------|
| 0 | Last_Block | 1 = final block |
| 2:1 | Block_Type | 0=Raw, 1=RLE, 2=Compressed |
| 23:3 | Block_Size | payload size in bytes |

The compressor tries each block type in order: **RLE** (all bytes identical, 4 bytes total), **Compressed** (LZ77 + FSE, only if smaller than raw), **Raw** (verbatim fallback). Blocks are capped at 128 KB; larger inputs are split automatically.

### Compressed block structure

```
[Literals section]        — raw literal bytes with a length header
[Sequence count]          — 1–3 bytes
[Symbol compression modes] — 0x00 = all-Predefined FSE
[FSE bitstream]           — backward-written bit stream
```

### FSE encoding

ZStd uses **predefined FSE tables** (RFC 8878 Appendix B) so no table descriptions need to be transmitted. The decoder reconstructs the same tables from fixed probability distributions for three streams:

- **LL** (Literal Length) — how many literal bytes precede each match
- **ML** (Match Length) — how long each LZ77 copy is
- **OF** (Offset) — how far back in the output the copy starts

The FSE bitstream is written **backwards**: the encoder writes from the last sequence to the first, and the decoder reads back in forward order. This is implemented via `RevBitWriter` / `RevBitReader`.

## Running tests

```bash
cd tests && LUA_PATH="../src/?.lua;../src/?/init.lua;../../lzss/src/?.lua;../../lzss/src/?/init.lua;;" busted . --verbose --pattern=test_
```

Or use the BUILD script from the package root:

```bash
luarocks show coding-adventures-lzss 1>/dev/null 2>/dev/null \
    || (cd ../lzss && luarocks make --local --deps-mode=none coding-adventures-lzss-0.1.0-1.rockspec)
luarocks make --local --deps-mode=none coding-adventures-zstd-0.1.0-1.rockspec
cd tests && LUA_PATH="../src/?.lua;../src/?/init.lua;../../lzss/src/?.lua;../../lzss/src/?/init.lua;;" \
    busted . --verbose --pattern=test_
```

## Test cases

| ID | Description | Key assertion |
|----|-------------|---------------|
| TC-1 | Empty input round-trip | `compress("") → decompress → ""` |
| TC-2 | Single byte `\x42` | Round-trip unchanged |
| TC-3 | All 256 byte values | Full byte-range round-trip |
| TC-4 | 1024 × `'A'` (RLE block) | Round-trip + compressed < 30 B |
| TC-5 | Repeated pangram (1125 B) | Round-trip + compressed < 80 % |
| TC-6 | 512 LCG pseudo-random bytes | Round-trip unchanged |
| TC-7 | 200 KB × `0xAB` (two RLE blocks) | Round-trip + compressed < 100 B |
| TC-8 | 300 KB repetitive text | Multi-block round-trip + compressed < 10 % |
| TC-9 | Bad magic `\x00\x00\x00\x00` | `decompress` raises error |
| TC-10 | Magic only (4 bytes, no FHD) | `decompress` raises error |
