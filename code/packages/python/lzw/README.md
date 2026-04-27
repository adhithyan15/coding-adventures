# coding-adventures-lzw

LZW (Lempel-Ziv-Welch, 1984) lossless compression — CMP03 in the coding-adventures series.

## What Is LZW?

LZW is LZ78 with a **pre-seeded dictionary**: all 256 single-byte sequences are added before
encoding begins (codes 0–255). This eliminates LZ78's mandatory `next_char` byte — every
symbol in the output is already in the dictionary, so the encoder can emit pure codes.

With only codes to transmit, LZW uses **variable-width bit-packing**: codes start at 9 bits
and grow as the dictionary expands. This is exactly how GIF compression works.

```
LZ78 token: (dict_index: u16, next_char: u8) = 3 bytes per token
LZW token:  code: u16, packed at 9–16 bits each   ← strictly smaller
```

## Usage

```python
from coding_adventures_lzw import compress, decompress

data = b"TOBEORNOTTOBEORTOBEORNOT"
compressed = compress(data)
assert decompress(compressed) == data
```

## Wire Format (CMP03)

```
Bytes 0–3:  original_length (big-endian uint32)
Bytes 4+:   variable-width codes, LSB-first bit-packed
              - code_size starts at 9 bits
              - grows when next_code crosses 2^k
              - first code is always CLEAR (256)
              - last code is always STOP  (257)
```

## The Tricky Token

LZW has a famous decoder edge case: receiving a code that hasn't been added to the dictionary
yet. This happens when the input contains a pattern of the form `xyx...x`:

```python
# "AAAAAAA" → codes: CLEAR, 65, 258, 259, 65, STOP
# When the decoder sees code 258 for the first time, it hasn't added 258 yet.
# Solution: entry = dict[prev_code] + bytes([dict[prev_code][0]])
```

## Series

```
CMP00 (LZ77,    1977) — Sliding-window backreferences
CMP01 (LZ78,    1978) — Explicit dictionary (trie)
CMP02 (LZSS,    1982) — LZ77 + flag bits; no wasted literals
CMP03 (LZW,     1984) — LZ78 + pre-initialized dict; powers GIF  ← this package
CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE
CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib
```
