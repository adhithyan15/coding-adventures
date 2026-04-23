# lz77

Pure F# LZ77 compression for the CMP00 sliding-window foundation.

## What It Includes

- Byte-array encoder and decoder using classic `(offset, length, nextChar)` tokens
- Overlap-safe decoding for self-referential backreferences
- Teaching-oriented token serialisation helpers and one-shot `Compress` / `Decompress` entry points

## Example

```fsharp
open System.Text
open CodingAdventures.Lz77.FSharp

let compressed = Lz77.Compress(Encoding.UTF8.GetBytes("ABABABAB"))
let roundTrip = Encoding.UTF8.GetString(Lz77.Decompress compressed)

printfn "%s" roundTrip
```

## Development

```bash
bash BUILD
```
