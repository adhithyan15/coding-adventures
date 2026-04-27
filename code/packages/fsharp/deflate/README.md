# deflate

Pure F# DEFLATE compression for the repo's CMP05 teaching wire format.

## What It Includes

- Composition of the native `.NET` `lzss` and `huffman-tree` packages
- Combined literal/length and distance Huffman coding
- CMP05 header and code-length table encoding
- One-shot `Compress` and `Decompress` helpers

## Example

```fsharp
open System.Text
open CodingAdventures.Deflate.FSharp

let compressed = Deflate.Compress(Encoding.UTF8.GetBytes("AABCBBABC"))
let roundTrip = Encoding.UTF8.GetString(Deflate.Decompress compressed)

printfn "%s" roundTrip
```

## Development

```bash
bash BUILD
```
