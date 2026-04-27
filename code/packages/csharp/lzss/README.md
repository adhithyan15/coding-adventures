# lzss

Pure C# LZSS compression for the CMP02 flagged sliding-window refinement.

## What It Includes

- Literal and match token types over the classic LZSS sliding-window model
- CMP02 block-flag serialisation with `originalLength` and `blockCount` headers
- One-shot `Compress` / `Decompress` helpers over the pure token API

## Example

```csharp
using System.Text;
using CodingAdventures.Lzss;

var compressed = Lzss.Compress(Encoding.UTF8.GetBytes("ABABAB"));
var roundTrip = Encoding.UTF8.GetString(Lzss.Decompress(compressed));

Console.WriteLine(roundTrip); // ABABAB
```

## Development

```bash
bash BUILD
```
