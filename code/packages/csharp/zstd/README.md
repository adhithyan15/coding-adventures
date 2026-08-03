# zstd

Pure C# implementation of the repository's CMP07 educational Zstandard format.

## What It Includes

- Zstandard magic, frame-header, content-size, and 128 KiB block framing
- Raw, run-length encoded, and compressed blocks
- Raw literal sections and predefined FSE sequence tables
- LZSS-generated literal and match sequences through the native C# `lzss` package
- A 256 MiB decompression limit and strict malformed-frame checks

This package follows the repository's established CMP07 teaching format. It is
an educational RFC 8878 subset: it does not decode arbitrary Zstandard streams
that use compressed literals or custom/repeat FSE tables.

The predefined-FSE sequences codec (table construction, per-sequence field
order, and the last-sequence state-init special case) is verified against the
real `zstd` CLI via `dotnet test` (see `Tc9CliInterop` and
`RepeatingPatternCliInterop` in the test project) — a same-codebase
round-trip test alone cannot catch a systematic, symmetric protocol
deviation, since the encoder and decoder would simply agree with each other
on the wrong convention. See `CHANGELOG.md` and lessons.md Lesson 96.

## Example

```csharp
using System.Text;
using CodingAdventures.Zstd;

var input = Encoding.UTF8.GetBytes("the quick brown fox");
var compressed = Zstd.Compress(input);
var roundTrip = Encoding.UTF8.GetString(Zstd.Decompress(compressed));
```

## Development

```bash
bash BUILD
```
