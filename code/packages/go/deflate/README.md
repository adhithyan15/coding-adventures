# deflate (Go)

**CMP05 — DEFLATE lossless compression (1996)**

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) educational computing stack.

## What Is DEFLATE?

DEFLATE combines LZSS back-reference tokenization with dual canonical Huffman coding. It powers ZIP, gzip, PNG, and HTTP/2 HPACK compression.

## Usage

```go
import deflate "github.com/adhithyan15/coding-adventures/code/packages/go/deflate"

data := []byte("hello hello hello world")
compressed, err := deflate.Compress(data)
// ...
original, err := deflate.Decompress(compressed)
```

## Wire Format

```
[4B] original_length    big-endian uint32
[2B] ll_entry_count     big-endian uint16
[2B] dist_entry_count   big-endian uint16
[ll_entry_count × 3B]   (symbol uint16 BE, code_length uint8)
[dist_entry_count × 3B] same format
[remaining bytes]       LSB-first packed bit stream
```

## Dependencies

- `lzss` — LZSS tokenizer (CMP02)
- `huffman-tree` — Huffman tree builder (DT27)
- `heap` — min-heap used by huffman-tree (DT27, transitive)
