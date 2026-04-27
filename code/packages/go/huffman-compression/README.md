# coding-adventures-huffman-compression (Go)

Huffman lossless compression (CMP04) in the coding-adventures series.

## Usage

```go
import "github.com/adhithyan15/coding-adventures/code/packages/go/huffman-compression"

data := []byte("AAABBC")
compressed, err := huffman.Compress(data)
if err != nil {
    // handle error
}
original, err := huffman.Decompress(compressed)
```

## Wire Format (CMP04)

```
Bytes 0–3:    original_length  (big-endian uint32)
Bytes 4–7:    symbol_count     (big-endian uint32)
Bytes 8–8+2N: code-length table — N entries × 2 bytes:
                [0] symbol value  (uint8)
                [1] code length   (uint8)
              Sorted by (code_length, symbol_value) ascending.
Bytes 8+2N+:  bit stream — LSB-first packed, zero-padded to byte boundary.
```

## How It Works

1. Count byte frequencies in the input.
2. Build a Huffman tree (via DT27 `huffman-tree`) — minimum-entropy prefix-free codes.
3. Derive canonical codes (DEFLATE-style): only code lengths are stored, not the tree.
4. Pack the encoded bit stream LSB-first.
5. Decompress by reconstructing canonical codes from lengths, then prefix-matching.

## Dependencies

- [`huffman-tree`](../huffman-tree) — DT27 Huffman tree (provides `Build`, `CanonicalCodeTable`, `DecodeAll`)

## Series

```
CMP00 (LZ77,    1977) — Sliding-window backreferences
CMP01 (LZ78,    1978) — Explicit dictionary (trie)
CMP02 (LZSS,    1982) — LZ77 + flag bits
CMP03 (LZW,     1984) — LZ78 + pre-initialized dict; powers GIF
CMP04 (Huffman, 1952) — Entropy coding                           ← this package
CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib
```
