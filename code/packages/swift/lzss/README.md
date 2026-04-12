# lzss — LZSS Lossless Compression Algorithm (Swift)

LZSS (Lempel-Ziv-Storer-Szymanski, 1982) sliding-window compression with flag-bit token
disambiguation. Part of the CMP compression series in the coding-adventures monorepo.

## In the Series

| Spec  | Algorithm      | Year | Key Concept                              |
|-------|----------------|------|------------------------------------------|
| CMP00 | LZ77           | 1977 | Sliding-window backreferences            |
| CMP01 | LZ78           | 1978 | Explicit dictionary (trie), no window    |
| CMP02 | **LZSS**       | 1982 | LZ77 + flag bits, no wasted literals ← you are here |
| CMP03 | LZW            | 1984 | Pre-initialized dictionary; powers GIF  |
| CMP04 | Huffman Coding | 1952 | Entropy coding; prerequisite for DEFLATE |
| CMP05 | DEFLATE        | 1996 | LZ77 + Huffman; ZIP/gzip/PNG/zlib        |

## Usage

```swift
import LZSS

let data       = Array("hello hello hello world".utf8)
let compressed = compress(data)
let original   = decompress(compressed)  // == data

// Token-level API
let tokens = encode(data)
let result = decode(tokens)

// Custom parameters
let tokens2 = encode(data, windowSize: 2048, maxMatch: 128, minMatch: 3)
```

## API

| Function | Description |
|----------|-------------|
| `encode(_:windowSize:maxMatch:minMatch:)` | Encode to token stream |
| `decode(_:originalLength:)` | Decode token stream |
| `compress(_:windowSize:maxMatch:minMatch:)` | Encode + serialise to CMP02 |
| `decompress(_:)` | Deserialise + decode |
| `serialiseTokens(_:originalLength:)` | Serialise token list to binary |
| `deserialiseTokens(_:)` | Deserialise binary; returns `(tokens, originalLength)` |

### Tokens

```swift
public enum Token: Equatable {
    case literal(UInt8)
    case match(offset: UInt16, length: UInt8)
}
```

### Parameters

| Parameter   | Default | Meaning |
|-------------|---------|---------|
| windowSize  | 4096    | Maximum lookback distance. |
| maxMatch    | 255     | Maximum match length. |
| minMatch    | 3       | Minimum match length for a Match token. |

## Development

```bash
swift test --enable-code-coverage
```
