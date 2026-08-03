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

Output is verified interoperable with the real `zstd` CLI: three xUnit tests
(`TC-9: ...`) shell out to a `zstd` binary on `PATH`, compressing/decompressing
across the F#-implementation boundary in both directions, including an input
large enough to cross the sequence-count wire encoding's 128-sequence
boundary. They're skipped gracefully (no assertions run, not failed) if
`zstd` isn't installed.

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
