# brotli

Pure C# Brotli-style compression for the CMP06 context-aware insert-copy stage.

## What It Includes

- The CMP06 insert-and-copy command table with sentinel handling
- Four literal-context Huffman trees keyed off the previous emitted byte
- An in-language sliding-window matcher up to the CMP06 `65535` byte window
- Canonical Huffman wire-format encoding for ICC, distance, and literal tables
- One-shot `Compress` and `Decompress` helpers over the pure codec

## Example

```csharp
using System.Text;
using CodingAdventures.Brotli;

var compressed = Brotli.Compress(Encoding.UTF8.GetBytes("abc123ABC"));
var roundTrip = Encoding.UTF8.GetString(Brotli.Decompress(compressed));

Console.WriteLine(roundTrip); // abc123ABC
```

## Development

```bash
bash BUILD
```
