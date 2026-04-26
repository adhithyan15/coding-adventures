# huffman-compression

Pure F# CMP04 Huffman lossless compression for byte arrays.

## What It Includes

- Canonical Huffman compression backed by the existing F# `huffman-tree` package
- CMP04-compatible wire format: big-endian length fields, sorted code-length entries, and LSB-first packed bits
- Graceful empty/null input handling and deterministic output for reproducible teaching vectors

## Example

```fsharp
open System.Text
open CodingAdventures.HuffmanCompression.FSharp

let original = Encoding.ASCII.GetBytes "AAABBC"
let compressed = HuffmanCompression.Compress original
let recovered = HuffmanCompression.Decompress compressed
```

## Development

```bash
bash BUILD
```
