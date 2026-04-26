# swift/zstd — CMP07: Zstandard Compression

A pure-Swift implementation of the Zstandard (ZStd) lossless compression format
(RFC 8878).  Part of the *coding-adventures* educational computing stack.

## What it does

`Zstd` compresses and decompresses data using the ZStd frame format:

- **LZ77 back-references** via the `LZSS` package (32 KB window, max match 255).
- **FSE (Finite State Entropy)** coding for sequence descriptors using the
  predefined distributions from RFC 8878 Appendix B.
- **Raw** and **RLE** block types for incompressible and uniform data.
- **256 MiB decompression cap** — guards against decompression-bomb attacks.

Output is standards-compliant: it can be decompressed by the `zstd` CLI or any
conforming decoder.

## How it fits in the stack

```
CMP00 (LZ77)    — Sliding-window back-references
CMP01 (LZ78)    — Explicit dictionary (trie)
CMP02 (LZSS)    — LZ77 + flag bits              ← dependency
CMP03 (LZW)     — LZ78 + pre-initialised alphabet; GIF
CMP04 (Huffman) — Entropy coding
CMP05 (DEFLATE) — LZ77 + Huffman; ZIP/gzip/PNG/zlib
CMP06 (Brotli)  — DEFLATE + context modelling + static dict
CMP07 (ZStd)    — LZ77 + FSE; high ratio + speed  ← this package
```

## Usage

```swift
import Zstd

// Compress
let original = Array("hello, ZStd!".utf8)
let compressed = compress(original)

// Decompress
let recovered = try decompress(compressed)
assert(recovered == original)
```

## API

```swift
/// Compress data to a valid ZStd frame (RFC 8878).
public func compress(_ data: [UInt8]) -> [UInt8]

/// Decompress a ZStd frame.
/// Throws ZstdError on malformed, truncated, or unsupported input.
public func decompress(_ data: [UInt8]) throws -> [UInt8]

public enum ZstdError: Error {
    case frameTooShort
    case badMagic(UInt32)
    case blockTruncated
    case reservedBlockType
    case outputLimitExceeded
    case bitstreamEmpty
    case bitstreamNoSentinel
    case unsupportedFSEModes
    case invalidOffset(UInt32, Int)
    case decodingError(String)
}
```

## Block types

| Condition               | Block type   | Cost                       |
|-------------------------|--------------|----------------------------|
| All bytes identical     | RLE          | 4 bytes (header + 1 byte)  |
| LZ77+FSE < input        | Compressed   | header + compressed bytes  |
| Otherwise               | Raw          | header + verbatim bytes    |

## Frame layout

```
┌────────┬─────┬────────────────────┬────────┬──────────────────┐
│ Magic  │ FHD │ Frame_Content_Size │ Blocks │ [Checksum]       │
│ 4B LE  │ 1B  │ 8B LE              │  ...   │ not written      │
└────────┴─────┴────────────────────┴────────┴──────────────────┘
```

Each block has a 3-byte header:
- Bit 0: Last_Block flag
- Bits [2:1]: Block_Type (Raw=00, RLE=01, Compressed=10, Reserved=11)
- Bits [23:3]: Block_Size

## Running tests

```bash
swift test --enable-code-coverage --verbose
# or via the BUILD script:
bash BUILD
```

## Limitations

- Only **Predefined FSE mode** is supported for decompression (no per-frame
  Huffman or custom FSE tables).  Frames produced by other encoders that use
  compressed FSE tables will throw `ZstdError.unsupportedFSEModes`.
- **Raw literals only** (no Huffman-coded literals section).
- Maximum decompressed output: 256 MiB.
- No streaming API (single-shot only).
- No checksum verification (Content_Checksum_Flag ignored on decode).

## Specification

`code/specs/CMP07-zstd.md` — RFC 8878 summary with implementation notes.
