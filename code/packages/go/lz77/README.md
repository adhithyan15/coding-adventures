# lz77 — LZ77 Lossless Compression Algorithm (Go)

LZ77 sliding-window compression algorithm (Lempel & Ziv, 1977). Part of the CMP compression series in the coding-adventures monorepo.

## What Is LZ77?

LZ77 replaces repeated byte sequences with compact backreferences into a sliding window of recently seen data. It is the foundation of DEFLATE, gzip, PNG, and zlib.

A token `(Offset, Length, NextChar)` means:
- **Offset**: how many bytes back the match begins (1 = immediately before)
- **Length**: how many bytes the match covers (0 = no match, emit literal)
- **NextChar**: the literal byte that follows the match

## How It Works

```
Input: A B A B A B A B
       ↓ ↓
       First two bytes → literal tokens
                 ↓ → backreference (Offset=2, Length=5, NextChar='B')
```

The encoder scans left-to-right, searching a `windowSize`-byte buffer for the longest match. Matches of length ≥ `minMatch` become backreferences; shorter ones emit literals.

The decoder must copy byte-by-byte (not bulk copy) to handle overlapping matches where `Offset < Length`.

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

```go
import lz77 "github.com/adhithyan15/coding-adventures/code/packages/go/lz77"

// One-shot compression / decompression
data := []byte("hello hello hello world")
compressed := lz77.Compress(data, 4096, 255, 3)
original := lz77.Decompress(compressed)

// Token-level API
tokens := lz77.Encode(data, 4096, 255, 3)
decoded := lz77.Decode(tokens, nil)

// Inspect tokens
for _, tok := range tokens {
    // tok.Offset, tok.Length, tok.NextChar
}
```

## API

| Function | Signature | Description |
|----------|-----------|-------------|
| `Encode` | `(data []byte, windowSize, maxMatch, minMatch int) []Token` | Encode to token stream |
| `Decode` | `(tokens []Token, initialBuffer []byte) []byte` | Decode token stream |
| `Compress` | `(data []byte, windowSize, maxMatch, minMatch int) []byte` | Encode + serialise |
| `Decompress` | `(data []byte) []byte` | Deserialise + decode |

### Token

```go
type Token struct {
    Offset   uint16
    Length   uint8
    NextChar byte
}
```

### Parameters

| Parameter  | Default | Meaning |
|------------|---------|---------|
| windowSize | 4096    | Maximum lookback distance. Larger = better compression, more memory. |
| maxMatch   | 255     | Maximum match length. Limited by the uint8 serialisation field. |
| minMatch   | 3       | Minimum match length before emitting a backreference. |

## Development

```bash
go test ./... -v -cover
```

Coverage target: 95%+ (currently 98.3%).
