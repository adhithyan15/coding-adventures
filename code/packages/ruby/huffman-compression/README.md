# coding-adventures-huffman-compression

**CMP04** — Huffman lossless compression using canonical Huffman codes.

Part of the [coding-adventures](https://github.com/coding-adventures) monorepo compression series.

## What It Does

Compresses and decompresses byte strings using Huffman coding (1952), the entropy-coding algorithm
that underpins DEFLATE, gzip, PNG, and ZIP.  Frequent bytes get short bit-codes; rare bytes get
long ones.  The result is provably optimal for the given byte-frequency distribution.

This package delegates tree construction to the `coding-adventures-huffman-tree` (DT27) gem and
handles the wire format, bit-packing, and decompression logic.

## CMP04 Wire Format

```
Bytes 0–3:    original_length  (big-endian uint32)
Bytes 4–7:    symbol_count     (big-endian uint32)
Bytes 8–8+2N: code-lengths table — N × 2 bytes: [symbol_uint8, length_uint8]
              Sorted by (code_length, symbol_value) ascending.
Bytes 8+2N+:  bit stream — LSB-first packed, zero-padded to byte boundary.
```

## Usage

```ruby
require "coding_adventures_huffman_compression"

data       = "the quick brown fox jumps over the lazy dog"
compressed = CodingAdventures::HuffmanCompression.compress(data)
original   = CodingAdventures::HuffmanCompression.decompress(compressed)

puts original == data.b  # => true
puts "#{data.bytesize} → #{compressed.bytesize} bytes"
```

## Where It Fits

| ID     | Algorithm | Year | Description                          |
|--------|-----------|------|--------------------------------------|
| CMP00  | LZ77      | 1977 | Sliding-window backreferences        |
| CMP01  | LZ78      | 1978 | Explicit dictionary (trie)           |
| CMP02  | LZSS      | 1982 | LZ77 + flag bits                     |
| CMP03  | LZW       | 1984 | LZ78 + pre-initialized dict (GIF)    |
| CMP04  | Huffman   | 1952 | **Entropy coding** (this package)    |
| CMP05  | DEFLATE   | 1996 | LZ77 + Huffman (ZIP/gzip/PNG)        |

## Dependencies

- `coding-adventures-huffman-tree` (~> 0.1) — DT27 min-heap Huffman tree

## Development

```bash
bundle install
bundle exec rake test
bundle exec standardrb --no-fix lib/
```
