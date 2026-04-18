# lzw

Pure C# LZW compression for the CMP03 bit-packed dictionary stage.

## What It Includes

- Public LZW constants for the shared CMP03 code space
- `BitWriter` and `BitReader` helpers for LSB-first variable-width code streams
- `EncodeCodes` and `DecodeCodes` for code-level teaching traces
- `PackCodes` and `UnpackCodes` for the CMP03 wire format
- One-shot `Compress` and `Decompress` helpers over the pure codec

## Example

```csharp
using System.Text;
using CodingAdventures.Lzw;

var compressed = Lzw.Compress(Encoding.UTF8.GetBytes("ABABAB"));
var roundTrip = Encoding.UTF8.GetString(Lzw.Decompress(compressed));

Console.WriteLine(roundTrip); // ABABAB
```

## Development

```bash
bash BUILD
```
