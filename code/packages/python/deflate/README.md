# coding-adventures-deflate

**CMP05 — DEFLATE lossless compression algorithm (1996)**

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) educational computing stack.

## What Is DEFLATE?

DEFLATE is the dominant general-purpose lossless compression algorithm. It powers ZIP, gzip, PNG, and HTTP/2 header compression. Implemented in this package from scratch using two building blocks from earlier in the series:

1. **LZSS tokenization** (CMP02) — replaces repeated substrings with back-references into a 4096-byte sliding window.
2. **Dual Huffman coding** (DT27/CMP04) — entropy-codes the resulting token stream with two canonical Huffman trees.

```
Input bytes → LZSS tokens → Huffman-coded bit stream
              (literals    (LL tree for literals + lengths,
               + matches)   dist tree for offsets)
```

## Usage

```python
from coding_adventures_deflate import compress, decompress

data = b"hello hello hello world"
compressed = compress(data)
original = decompress(compressed)
assert original == data

# Inspect compression ratio
print(f"{len(data)} → {len(compressed)} bytes ({100*len(compressed)//len(data)}%)")
```

## How It Works

### Pass 1: LZSS Tokenization

The input is scanned left-to-right. At each position, we search the last 4096 bytes for the longest match. If a match of length ≥ 3 is found, it becomes a `Match(offset, length)` token. Otherwise, a `Literal(byte)` token is emitted.

```
"AABCBBABC":
cursor=0: no window → Literal('A')
cursor=1: 'A' in window, len=1 < 3 → Literal('A')
cursor=2: no match → Literal('B')
cursor=3: no match → Literal('C')
cursor=4: 'B', len=1 < 3 → Literal('B')
cursor=5: 'B', len=1 < 3 → Literal('B')
cursor=6: 'A','B','C' matches window[1..3] → Match(offset=5, length=3)
```

### Pass 2: Dual Huffman Coding

Two separate Huffman trees are built from the token stream frequencies:

**LL (Literal/Length) tree**: covers:
- Symbols 0–255: literal byte values
- Symbol 256: end-of-data marker
- Symbols 257–284: length codes (each covering a range via extra bits)

**Distance tree**: covers codes 0–23 for offsets 1–4096.

Length codes use **extra bits**: symbol 266 covers lengths 13–14 (1 extra bit), symbol 274 covers lengths 43–50 (3 extra bits), etc. This compresses the length alphabet from 253 symbols down to 28.

### Wire Format

```
[4B] original_length    big-endian uint32
[2B] ll_entry_count     big-endian uint16
[2B] dist_entry_count   big-endian uint16 (0 if no matches)
[ll_entry_count × 3B]   (symbol uint16 BE, code_length uint8), sorted
[dist_entry_count × 3B] same format
[remaining bytes]       LSB-first packed bit stream
```

## Dependencies

- [`coding-adventures-lzss`](../lzss/) — LZSS tokenizer (CMP02)
- [`coding-adventures-huffman-tree`](../huffman-tree/) — Huffman tree builder (DT27)

## Installation

```bash
pip install -e .
# or with uv:
uv pip install -e .
```

## Development

```bash
uv pip install -e .[dev]
pytest tests/ -v
```

## Series

```
CMP00 (LZ77,    1977) — Sliding-window backreferences.
CMP01 (LZ78,    1978) — Explicit dictionary (trie).
CMP02 (LZSS,    1982) — LZ77 + flag bits; no wasted literals.
CMP03 (LZW,     1984) — LZ78 + pre-initialized dict; powers GIF.
CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE.
CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.  ← THIS PACKAGE
```
