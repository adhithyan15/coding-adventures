# huffman-compression — CMP04

Swift implementation of Huffman compression (1952), part of the
[coding-adventures](https://github.com/adhithyan15/coding-adventures)
educational computing stack.

## What Is Huffman Compression?

Huffman compression assigns variable-length bit codes to symbols based on their
frequency. Frequent symbols get short codes; rare symbols get long codes. The
result is the provably optimal prefix-free code for the given symbol distribution.

This module implements **canonical Huffman codes** (DEFLATE-style): only the
code-lengths table is stored in the wire format, not the tree structure. The
decoder reconstructs the exact same codes from the lengths alone.

```
Series:
  CMP00 (LZ77,    1977) — Sliding-window backreferences.
  CMP01 (LZ78,    1978) — Explicit dictionary (trie).
  CMP02 (LZSS,    1982) — LZ77 + flag bits; no wasted literals.
  CMP03 (LZW,     1984) — LZ78 + pre-initialised alphabet; GIF.
  CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE.  ← YOU ARE HERE
  CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib standard.
```

## Wire Format (CMP04)

```
Bytes 0–3:    original_length  (big-endian uint32)
Bytes 4–7:    symbol_count     (big-endian uint32)
Bytes 8–8+2N: code-lengths table — N entries, each 2 bytes:
                [0] symbol value  (uint8, 0–255)
                [1] code length   (uint8, 1–16)
              Sorted by (code_length, symbol_value) ascending.
Bytes 8+2N+:  bit stream — packed LSB-first, zero-padded to byte boundary.
```

## Usage

```swift
import HuffmanCompression

// Compress
let original: [UInt8] = Array("hello world".utf8)
let compressed = try compress(original)

// Decompress
let restored = try decompress(compressed)
// restored == original ✓
```

## Example

For "AAABBC" (6 bytes):

| Symbol | Frequency | Canonical Code | Length |
|--------|-----------|----------------|--------|
| A (65) | 3         | `0`            | 1      |
| B (66) | 2         | `10`           | 2      |
| C (67) | 1         | `11`           | 2      |

Bit stream: `A A A B B C` → `0 0 0 10 10 11` = `000101011` (9 bits)  
Packed LSB-first: `[0xA8, 0x01]`

Full compressed output: `[0,0,0,6, 0,0,0,3, 65,1, 66,2, 67,2, 0xA8,0x01]`

## Dependencies

- [`HuffmanTree`](../huffman-tree) (DT27) — provides the tree-building algorithm
  and `canonicalCodeTable()`.

## Running Tests

```bash
# macOS
xcrun swift test --enable-code-coverage --verbose

# Linux
swift test --enable-code-coverage --verbose
```
