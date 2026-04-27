# deflate

Pure C# DEFLATE compression for the repo's CMP05 teaching wire format.

## What It Includes

- Composition of the native `.NET` `lzss` and `huffman-tree` packages
- Combined literal/length and distance Huffman coding
- CMP05 header and code-length table encoding
- One-shot `Compress` and `Decompress` helpers

## Example

```csharp
using System.Text;
using CodingAdventures.Deflate;

var compressed = Deflate.Compress(Encoding.UTF8.GetBytes("AABCBBABC"));
var roundTrip = Encoding.UTF8.GetString(Deflate.Decompress(compressed));

Console.WriteLine(roundTrip); // AABCBBABC
```

## Development

```bash
bash BUILD
```
