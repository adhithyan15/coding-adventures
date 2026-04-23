# coding-adventures-lzw (Swift)

LZW (Lempel-Ziv-Welch, 1984) lossless compression — CMP03 in the coding-adventures
compression series.

## What Is LZW?

LZW is LZ78 with a pre-seeded dictionary: all 256 single-byte sequences are present
before encoding begins (codes 0–255). This eliminates the explicit "next character"
that LZ78 requires — every symbol is already in the dictionary, so the encoder emits
pure codes.

With only codes to transmit, LZW uses **variable-width bit-packing**: codes start at
9 bits and grow as the dictionary expands (up to 16 bits). This is exactly how GIF works.

## Reserved Codes

| Code | Meaning                                |
|------|----------------------------------------|
| 0–255 | Pre-seeded single-byte literals      |
| 256  | `clearCode` — reset to initial state  |
| 257  | `stopCode`  — end of stream           |
| 258+ | Dynamically added entries             |

## Wire Format (CMP03)

```
Bytes 0–3:  original_length (big-endian UInt32)
Bytes 4+:   variable-width codes, bit-packed LSB-first
```

Codes start at 9 bits. `code_size` increases when `next_code` exceeds the current
power-of-2 boundary. A `clearCode` resets `code_size` to 9.

## Usage

```swift
import LZW

let original: [UInt8] = Array("hello, world!".utf8)
let compressed   = compress(original)
let decompressed = decompress(compressed)
assert(decompressed == original)
```

## The Tricky Token

When the decoder receives a code equal to `next_code` (not yet added), the sequence
has the form `xyx...x`. The fix:

```
entry = dict[prevCode] + [dict[prevCode][0]]
```

This edge case is exercised by the `"AAAAAAA"` test vector.

## The Compression Series

| ID    | Algorithm | Year | Description                                  |
|-------|-----------|------|----------------------------------------------|
| CMP00 | LZ77      | 1977 | Sliding-window back-references               |
| CMP01 | LZ78      | 1978 | Explicit dictionary (trie)                   |
| CMP02 | LZSS      | 1982 | LZ77 + flag bits; no wasted literals         |
| **CMP03** | **LZW** | **1984** | **LZ78 + pre-initialized dict; GIF**    |
| CMP04 | Huffman   | 1952 | Entropy coding; prerequisite for DEFLATE     |
| CMP05 | DEFLATE   | 1996 | LZ77 + Huffman; ZIP/gzip/PNG/zlib            |

## Running Tests

```sh
swift test --verbose
```

Requires Swift 5.7+.
