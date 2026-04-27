# lz78

Pure C# LZ78 compression for the CMP01 explicit-dictionary foundation.

## What It Includes

- Byte-array encoder and decoder using `(dictIndex, nextChar)` tokens
- CMP01 teaching-format serialisation helpers with original-length metadata
- One-shot `Compress` / `Decompress` helpers over the pure token API

## Example

```csharp
using System.Text;
using CodingAdventures.Lz78;

var compressed = Lz78.Compress(Encoding.UTF8.GetBytes("ABABAB"));
var roundTrip = Encoding.UTF8.GetString(Lz78.Decompress(compressed));

Console.WriteLine(roundTrip); // ABABAB
```

## Development

```bash
bash BUILD
```
