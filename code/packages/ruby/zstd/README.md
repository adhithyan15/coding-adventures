# coding_adventures_zstd

ZStd (Zstandard, RFC 8878) lossless compression from scratch — CMP07.

## What Is This?

Zstandard was created by Yann Collet at Facebook in 2015 and standardised as
[RFC 8878](https://www.rfc-editor.org/rfc/rfc8878). It delivers high compression
ratios at decompression speeds that rival memcpy, making it the default
compression algorithm for Facebook's databases, Linux kernel firmware, and many
modern storage systems.

This gem is a pure-Ruby teaching implementation of the full compress/decompress
pipeline. Every component is explained with literate-programming comments.

## How It Fits in the Series

```
CMP00 (LZ77)     — Sliding-window back-references
CMP01 (LZ78)     — Explicit dictionary (trie)
CMP02 (LZSS)     — LZ77 + flag bits  ← dependency
CMP03 (LZW)      — LZ78 + pre-initialised alphabet; GIF
CMP04 (Huffman)  — Entropy coding
CMP05 (DEFLATE)  — LZ77 + Huffman; ZIP/gzip/PNG/zlib
CMP06 (Brotli)   — DEFLATE + context modelling + static dict
CMP07 (ZStd)     — LZ77 + FSE; high ratio + speed  ← this gem
```

## Architecture

ZStd combines three key techniques:

### 1. LZ77 Back-References (via LZSS)

LZSS (`coding_adventures_lzss`) finds repeated substrings in the input and
replaces them with (offset, length) back-references. For a 128 KB block of
English text repeated 100 times, almost every byte after the first copy is a
back-reference.

### 2. FSE (Finite State Entropy)

FSE is an Asymmetric Numeral System (ANS) codec invented by Jarek Duda. Where
Huffman coding requires an integer number of bits per symbol, FSE can use
fractional bits, approaching the Shannon entropy limit.

Each LZSS match produces a *sequence*: `(literal_length, match_length, offset)`.
The three fields are FSE-coded using *predefined* tables (RFC 8878 Appendix B),
so no per-frame table description is needed.

### 3. Reverse Bitstream

ZStd's sequence section is encoded *backwards*: the last symbol is written first
so the decoder can read forward without buffering. A sentinel bit in the last byte
tells the decoder where the data ends.

## Frame Layout

```
┌────────┬─────┬──────────────────────┬────────┐
│ Magic  │ FHD │ Frame_Content_Size   │ Blocks │
│ 4 B LE │ 1 B │ 8 B (LE)            │ ...    │
└────────┴─────┴──────────────────────┴────────┘
```

Each block has a 3-byte header: `[last(1) | type(2) | size(21)]`.

Block types:
- **Raw** (00) — verbatim bytes, no compression
- **RLE** (01) — single byte repeated N times
- **Compressed** (10) — literals section + FSE sequence bitstream

## Usage

```ruby
require "coding_adventures_zstd"

# Compress
data       = "the quick brown fox jumps over the lazy dog " * 100
compressed = CodingAdventures::Zstd.compress(data)

# Decompress
original   = CodingAdventures::Zstd.decompress(compressed)
original == data  # => true

# Compression ratio
puts "#{compressed.bytesize} / #{data.bytesize} = #{compressed.bytesize * 100 / data.bytesize}%"
# => roughly 3% for highly repetitive text
```

## Installation

```ruby
# Gemfile
gem "coding_adventures_zstd", path: "code/packages/ruby/zstd"
gem "coding_adventures_lzss",  path: "code/packages/ruby/lzss"
```

## Tests

```
bundle exec rake test
```

54 tests covering:
- Empty input, single byte, all 256 byte values
- RLE detection and sizing (< 30 bytes for 1024 identical bytes)
- English prose with >= 20% compression
- LCG pseudo-random data round-trip
- Multi-block frames (200 KB, 300 KB)
- Bad magic detection
- Wire-format decoding (hand-crafted frames)
- Internal unit tests for FSE tables, RevBitWriter/Reader, literals section

## Key Data Structures

| Class / Method | Role |
|----------------|------|
| `RevBitWriter` | Backward bit accumulator with sentinel flush |
| `RevBitReader` | Backward bit reader with left-aligned 64-bit register |
| `build_decode_table` | FSE decode table from normalised distribution |
| `build_encode_tables` | FSE encode tables (delta_nb, delta_fs, state table) |
| `Zstd.compress` | Public compress: frame header + blocks |
| `Zstd.decompress` | Public decompress: validate magic, parse blocks |

## Security

- Decompressed output is capped at 256 MiB (`MAX_OUTPUT`) to prevent zip bombs.
- All slice operations are bounds-checked; truncated input raises `RuntimeError`.
- Bad magic raises immediately.
