# lz78 — LZ78 Lossless Compression Algorithm (Go)

LZ78 (Lempel & Ziv, 1978) explicit-dictionary compression. Part of the CMP compression series in the coding-adventures monorepo.

## In the Series

| Spec  | Algorithm      | Year | Key Concept                                  |
|-------|----------------|------|----------------------------------------------|
| CMP00 | LZ77           | 1977 | Sliding-window backreferences                |
| CMP01 | **LZ78**       | 1978 | Explicit dictionary (trie) ← you are here    |
| CMP02 | LZSS           | 1982 | LZ77 + flag bits, no wasted literals         |
| CMP03 | LZW            | 1984 | LZ78 + pre-initialized alphabet; powers GIF |
| CMP04 | Huffman Coding | 1952 | Entropy coding; prerequisite for DEFLATE     |
| CMP05 | DEFLATE        | 1996 | LZ77 + Huffman; ZIP/gzip/PNG/zlib            |

## Usage

```go
import lz78 "github.com/adhithyan15/coding-adventures/code/packages/go/lz78"

// One-shot compression / decompression
data := []byte("hello hello hello world")
compressed := lz78.Compress(data, 65536)
original   := lz78.Decompress(compressed)

// Token-level API
tokens := lz78.Encode(data, 65536)
decoded := lz78.Decode(tokens, len(data))
```

## API

| Function     | Signature                                            | Description              |
|--------------|------------------------------------------------------|--------------------------|
| `Encode`     | `([]byte, maxDictSize int) → []Token`                | Encode to token stream   |
| `Decode`     | `([]Token, originalLength int) → []byte`             | Decode token stream      |
| `Compress`   | `([]byte, maxDictSize int) → []byte`                 | Encode + serialise       |
| `Decompress` | `([]byte) → []byte`                                  | Deserialise + decode     |

### Token

```go
type Token struct {
    DictIndex uint16
    NextChar  byte
}
```

## Development

```bash
go test ./... -v -cover
```
