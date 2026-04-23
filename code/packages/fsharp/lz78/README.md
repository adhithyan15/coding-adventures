# lz78

Pure F# LZ78 compression for the CMP01 explicit-dictionary foundation.

## What It Includes

- Byte-array encoder and decoder using `(dictIndex, nextChar)` tokens
- CMP01 teaching-format serialisation helpers with original-length metadata
- One-shot `Compress` / `Decompress` helpers over the pure token API

## Example

```fsharp
open System.Text
open CodingAdventures.Lz78.FSharp

let compressed = Lz78.Compress(Encoding.UTF8.GetBytes("ABABAB"))
let roundTrip = Encoding.UTF8.GetString(Lz78.Decompress compressed)

printfn "%s" roundTrip
```

## Development

```bash
bash BUILD
```
