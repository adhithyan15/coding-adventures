# lzss

Pure F# LZSS compression for the CMP02 flagged sliding-window refinement.

## What It Includes

- Literal and match token types over the classic LZSS sliding-window model
- CMP02 block-flag serialisation with `originalLength` and `blockCount` headers
- One-shot `Compress` / `Decompress` helpers over the pure token API

## Example

```fsharp
open System.Text
open CodingAdventures.Lzss.FSharp

let compressed = Lzss.Compress(Encoding.UTF8.GetBytes("ABABAB"))
let roundTrip = Encoding.UTF8.GetString(Lzss.Decompress compressed)

printfn "%s" roundTrip
```

## Development

```bash
bash BUILD
```
