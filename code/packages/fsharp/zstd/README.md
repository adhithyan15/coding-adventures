# zstd

Pure F# implementation of the repository's CMP07 educational Zstandard format.

## What It Includes

- Zstandard magic, frame-header, content-size, and 128 KiB block framing
- Raw, run-length encoded, and compressed blocks
- Raw literal sections and predefined FSE sequence tables
- LZSS-generated literal and match sequences through the native F# `lzss` package
- A 256 MiB decompression limit and strict malformed-frame checks

This package follows the repository's established CMP07 teaching format. It is
an educational RFC 8878 subset: it does not decode arbitrary Zstandard streams
that use compressed literals or custom/repeat FSE tables.

## Example

```fsharp
open System.Text
open CodingAdventures.Zstd.FSharp

let input = Encoding.UTF8.GetBytes "the quick brown fox"
let compressed = Zstd.Compress input
let roundTrip = Zstd.Decompress compressed |> Encoding.UTF8.GetString
```

## Development

```bash
bash BUILD
```
