# CodingAdventures::Brotli (Perl)

**CMP06 — Brotli-style lossless compression (2013)**

This package implements the CodingAdventures CMP06 specification: a
Brotli-inspired compression algorithm that captures the three key innovations
of RFC 7932 while omitting the static dictionary to keep the implementation
tractable across all nine language targets.

## What Is Brotli?

Brotli (named after a Swiss bread roll) was developed at Google in 2013 by
Jyrki Alakuijärvi and Zoltán Szabadka. It became the standard compression
format for HTTP `Content-Encoding: br` responses and is used inside every
`.woff2` web font file. On typical web content it achieves 15–25% better
compression than gzip/DEFLATE.

## Three Key Innovations

### 1. Context-Dependent Literal Trees

DEFLATE uses one Huffman tree for all literal bytes. Brotli uses **four
trees**, one per *context bucket*:

| Bucket | Last byte was… |
|--------|----------------|
| 0      | space or punctuation (default/start) |
| 1      | a digit `'0'`–`'9'` |
| 2      | an uppercase letter `'A'`–`'Z'` |
| 3      | a lowercase letter `'a'`–`'z'` |

Because the distribution of the *next* byte strongly depends on what the
*previous* byte was, each tree can be highly specialized, giving shorter
average codes on structured text.

### 2. Insert-and-Copy Commands

Instead of separate "literal" and "back-reference" tokens (DEFLATE style),
Brotli bundles them into commands:

```
Command {
  insert_length  — how many raw literal bytes follow
  copy_length    — how many bytes to copy from history
  copy_distance  — how far back to look (1 = last byte)
}
```

The insert and copy lengths are encoded together as a single **insert-copy
code (ICC)** Huffman symbol, saving overhead.

### 3. 65535-Byte Sliding Window

CMP05 (DEFLATE) uses a 4096-byte window. CMP06 extends this to **65535
bytes**, letting the LZ matcher reference repetitions that are thousands of
bytes apart — common in HTML pages with repeated navigation, CSS, and
JavaScript boilerplate.

## Usage

```perl
use CodingAdventures::Brotli qw(compress decompress);

my $data       = "hello hello hello world";
my $compressed = compress($data);
my $original   = decompress($compressed);  # "hello hello hello world"
```

## Wire Format

```
Header (10 bytes):
  [4B] original_length      big-endian uint32
  [1B] icc_entry_count      entries in ICC code-length table (1–64)
  [1B] dist_entry_count     entries in dist code-length table (0–32)
  [1B] ctx0_entry_count     entries in literal tree 0 (space/punct)
  [1B] ctx1_entry_count     entries in literal tree 1 (digit)
  [1B] ctx2_entry_count     entries in literal tree 2 (uppercase)
  [1B] ctx3_entry_count     entries in literal tree 3 (lowercase)

ICC code-length table (icc_entry_count × 2 bytes):
  [1B] symbol (0–63)
  [1B] code_length (1–16)
  Sorted: (code_length ASC, symbol ASC)

Dist code-length table (dist_entry_count × 2 bytes):
  [1B] symbol (0–31)
  [1B] code_length (1–16)

Literal tree 0–3 tables (ctx_N_entry_count × 3 bytes each):
  [2B] symbol (byte value 0–255, big-endian uint16)
  [1B] code_length (1–16)

Bit stream (remaining bytes):
  LSB-first packed. Zero-padded to byte boundary.
```

## Dependencies

- `CodingAdventures::HuffmanTree` (DT27) — canonical Huffman tree builder

## Series Position

```
CMP00 (LZ77)     → CMP01 (LZ78)   → CMP02 (LZSS)
CMP03 (LZW)      → CMP04 (Huffman) → CMP05 (DEFLATE)
CMP06 (Brotli) ← YOU ARE HERE
CMP07 (Zstd)
```
