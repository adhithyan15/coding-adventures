# brotli (Swift)

**CMP06 — Brotli-inspired lossless compression (2013)**

## Overview

Brotli is a compression algorithm developed at Google that builds on DEFLATE
(CMP05) with three major innovations:

1. **Context-dependent literal trees** — instead of one Huffman tree for all
   literals, Brotli assigns each literal to one of 4 context buckets based on
   the preceding byte. Each bucket gets its own Huffman tree, exploiting the
   statistical structure of natural language.

2. **Insert-and-copy commands** — instead of DEFLATE's flat stream of "literal"
   and "back-reference" tokens, Brotli uses commands that bundle an insert run
   (raw literals) with a copy operation (back-reference). The lengths of both
   halves are encoded together in a single Huffman symbol.

3. **Larger sliding window** — 65535 bytes vs DEFLATE's 4096 bytes, allowing
   matches across much longer distances.

This is an educational implementation that captures these three innovations.
The static dictionary from RFC 7932 is omitted.

## Usage

```swift
import CodingAdventuresBrotli

let data = Array("hello hello hello world".utf8)
let compressed   = try Brotli.compress(data)
let decompressed = try Brotli.decompress(compressed)
// decompressed == data
```

## Wire Format

```
Header (10 bytes):
  [4B] original_length    — big-endian uint32
  [1B] icc_entry_count    — uint8 (1–64)
  [1B] dist_entry_count   — uint8 (0–32)
  [1B] ctx0_entry_count   — uint8
  [1B] ctx1_entry_count   — uint8
  [1B] ctx2_entry_count   — uint8
  [1B] ctx3_entry_count   — uint8

ICC code-length table   (icc_entry_count × 2 bytes: symbol uint8, code_length uint8)
Distance code-length    (dist_entry_count × 2 bytes: same)
Literal tree 0          (ctx0_entry_count × 3 bytes: symbol uint16 BE, code_length uint8)
Literal tree 1–3        (same structure)
Bit stream              (remaining bytes, LSB-first packed)
```

## Context Buckets

```
bucket 0 — space/punctuation (0x00–0x2F, 0x3A–0x40, 0x5B–0x60, 0x7B–0xFF)
bucket 1 — digit ('0'–'9')
bucket 2 — uppercase letter ('A'–'Z')
bucket 3 — lowercase letter ('a'–'z')
```

## How it fits in the stack

```
CMP00 (LZ77,    1977) → CMP01 (LZ78,     1978) → CMP02 (LZSS,  1982)
CMP03 (LZW,     1984) → CMP04 (Huffman,  1952) → CMP05 (DEFLATE, 1996)
CMP06 (Brotli,  2013) ← this package            → CMP07 (Zstd, 2016)
```

## Dependencies

- `CodingAdventuresHuffmanTree` (DT27) — canonical Huffman tree builder
