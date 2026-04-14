# coding_adventures_brotli (Elixir)

**CMP06 — Brotli lossless compression (2013)**

Part of the CodingAdventures compression series.

## What Is Brotli?

Brotli (RFC 7932) is a lossless compression algorithm developed at Google. It is
the dominant algorithm for HTTP `Content-Encoding: br` and the WOFF2 font format.
Brotli improves on DEFLATE (CMP05) in three key ways:

1. **Context-dependent literal trees** — four separate Huffman trees for literals,
   one per context bucket based on the preceding byte's character class.

2. **Insert-and-copy commands** — literals and back-references are bundled into
   single commands with a shared ICC Huffman symbol, reducing token overhead.

3. **Larger sliding window** — 65535 bytes vs DEFLATE's 4096 bytes.

This package implements the CMP06 educational subset: 4 literal context buckets,
64 ICC codes, 32 distance codes (up to 65535), and no static dictionary.

## Usage

```elixir
alias CodingAdventures.Brotli

data = "hello hello hello world"
compressed = Brotli.compress(data)
original = Brotli.decompress(compressed)
# original == data
```

Accepts binary or list of byte values:

```elixir
Brotli.compress(:binary.list_to_bin([104, 101, 108, 108, 111]))
Brotli.compress([104, 101, 108, 108, 111])
```

## Wire Format

```
Header (10 bytes):
  [4B] original_length       big-endian uint32
  [1B] icc_entry_count       entries in ICC table (1-64)
  [1B] dist_entry_count      entries in dist table (0-32)
  [1B] ctx0_entry_count      entries in literal tree 0
  [1B] ctx1_entry_count      entries in literal tree 1
  [1B] ctx2_entry_count      entries in literal tree 2
  [1B] ctx3_entry_count      entries in literal tree 3

ICC table (icc_entry_count × 2 bytes): symbol::8, code_length::8
Dist table (dist_entry_count × 2 bytes): symbol::8, code_length::8
Literal trees 0-3 (entry_count × 3 bytes each): symbol::16 BE, code_length::8
Bit stream: LSB-first packed bits, zero-padded to byte boundary.
```

## Algorithm Overview

**Compression:**

1. LZ matching with 65535-byte window (minimum match length 4).
2. Trailing literals with no following match become "flush literals".
3. Regular commands (each with a real ICC code) + flush literals after sentinel.
4. Huffman trees built from frequencies.
5. Bit stream: per-command `[ICC][insert extras][copy extras][literals][dist][dist extras]`,
   then `[ICC=63][flush literals]`.

**Decompression:**

1. Parse header, code-length tables, reconstruct canonical Huffman maps.
2. Decode ICC commands until sentinel (ICC=63).
3. After sentinel: read remaining flush literals until `original_length` bytes.

## Dependencies

- [`coding_adventures_huffman_tree`](../../elixir/huffman_tree/) (DT27) — canonical
  Huffman tree builder.

## Series Context

```
CMP02 (LZSS,    1982) — LZ77 + flag bits
CMP05 (DEFLATE, 1996) — LZSS + dual Huffman
CMP06 (Brotli,  2013) — Context modeling + ICC + large window ← this package
CMP07 (Zstd,    2016) — ANS/FSE + LZ4 matching
```
