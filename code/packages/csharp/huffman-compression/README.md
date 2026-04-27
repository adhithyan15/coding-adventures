# huffman-compression

Pure C# CMP04 Huffman lossless compression for byte arrays.

## What It Includes

- Canonical Huffman compression backed by the existing C# `huffman-tree` package
- CMP04-compatible wire format: big-endian length fields, sorted code-length entries, and LSB-first packed bits
- Graceful empty/null input handling and deterministic output for reproducible teaching vectors

## Example

```csharp
using CodingAdventures.HuffmanCompression;

var original = "AAABBC"u8.ToArray();
var compressed = HuffmanCompression.Compress(original);
var recovered = HuffmanCompression.Decompress(compressed);
```

## Development

```bash
bash BUILD
```
