# lz77

Pure C# LZ77 compression for the CMP00 sliding-window foundation.

## What It Includes

- Byte-array encoder and decoder using classic `(offset, length, nextChar)` tokens
- Overlap-safe decoding for self-referential backreferences
- Teaching-oriented token serialisation helpers and one-shot `Compress` / `Decompress` entry points

## Example

```csharp
using CodingAdventures.Lz77;
using System.Text;

var compressed = Lz77.Compress(Encoding.UTF8.GetBytes("ABABABAB"));
var roundTrip = Encoding.UTF8.GetString(Lz77.Decompress(compressed));

Console.WriteLine(roundTrip); // ABABABAB
```

## Development

```bash
bash BUILD
```
