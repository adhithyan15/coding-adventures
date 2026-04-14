# coding_adventures_huffman_compression

Huffman lossless compression — CMP04 in the coding-adventures series.

## What it does

Compresses and decompresses binary data using Huffman entropy coding. Symbols
that appear frequently get short bit codes; rare symbols get long codes. The
algorithm is provably optimal for independent symbol coding given a known
frequency distribution.

This package implements the CMP04 wire format, which stores only the canonical
code-length table (not the tree), making the compressed format self-contained
and decodable without any shared state.

## Dependency

Depends on `coding_adventures_huffman_tree` (DT27) for tree construction and
canonical code generation.

## Wire format

```
Bytes 0–3:    original_length  (big-endian uint32)
Bytes 4–7:    symbol_count     (big-endian uint32)
Bytes 8–8+2N: code-lengths table — N * 2 bytes, each {symbol::8, length::8}
              Sorted by (code_length, symbol_value) ascending.
Bytes 8+2N+:  bit stream — LSB-first, zero-padded to byte boundary.
```

## Usage

```elixir
alias CodingAdventures.HuffmanCompression

# Compress
compressed = HuffmanCompression.compress("hello hello hello")

# Decompress
"hello hello hello" = HuffmanCompression.decompress(compressed)

# Error on truncated input
{:error, :too_short} = HuffmanCompression.decompress(<<1, 2>>)
```

## Series position

    CMP00 (LZ77)    — Sliding-window backreferences.
    CMP01 (LZ78)    — Explicit dictionary.
    CMP02 (LZSS)    — LZ77 + flag bits.
    CMP03 (LZW)     — LZ78 + pre-initialised dict; GIF.
    CMP04 (Huffman) — Entropy coding. ← this package
    CMP05 (DEFLATE) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.
