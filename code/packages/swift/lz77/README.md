# lz77 — LZ77 Lossless Compression Algorithm (Swift)

LZ77 sliding-window compression algorithm (Lempel & Ziv, 1977). Part of the CMP compression series in the coding-adventures monorepo.

## In the Series

| Spec  | Algorithm      | Year | Key Concept                              |
|-------|----------------|------|------------------------------------------|
| CMP00 | **LZ77**       | 1977 | Sliding-window backreferences ← you are here |
| CMP01 | LZ78           | 1978 | Explicit dictionary (trie), no window    |
| CMP02 | LZSS           | 1982 | LZ77 + flag bits, no wasted literals     |
| CMP03 | LZW            | 1984 | Pre-initialized dictionary; powers GIF  |
| CMP04 | Huffman Coding | 1952 | Entropy coding; prerequisite for DEFLATE |
| CMP05 | DEFLATE        | 1996 | LZ77 + Huffman; ZIP/gzip/PNG/zlib        |

## Usage

```swift
import LZ77

// One-shot compression / decompression
let data: [UInt8] = Array("hello hello hello world".utf8)
let compressed = compress(data)
let original   = decompress(compressed)

// Token-level API
let tokens = encode(data)
let decoded = decode(tokens)

// Custom parameters
let tokens2 = encode(data, windowSize: 2048, maxMatch: 128, minMatch: 3)
```

## API

| Function | Signature | Description |
|----------|-----------|-------------|
| `encode` | `([UInt8], windowSize:, maxMatch:, minMatch:) → [Token]` | Encode to token stream |
| `decode` | `([Token], initialBuffer:) → [UInt8]` | Decode token stream |
| `compress` | `([UInt8], windowSize:, maxMatch:, minMatch:) → [UInt8]` | Encode + serialise |
| `decompress` | `([UInt8]) → [UInt8]` | Deserialise + decode |

### Token

```swift
public struct Token: Equatable {
    public let offset: UInt16
    public let length: UInt8
    public let nextChar: UInt8
}
```

### Parameters

| Parameter  | Default | Meaning |
|------------|---------|---------|
| windowSize | 4096    | Maximum lookback distance. |
| maxMatch   | 255     | Maximum match length. |
| minMatch   | 3       | Minimum match length for backreference. |

## Development

```bash
swift test
```

26 tests, 0 failures. Windows: Swift not supported on Windows runners (BUILD_windows prints skip).
