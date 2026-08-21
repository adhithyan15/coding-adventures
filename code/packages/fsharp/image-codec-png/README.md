# image-codec-png (F#)

`image-codec-png` is the native F# implementation of the bounded IC18 portable
PNG profile. It encodes RGBA8 `PixelContainer` values and decodes
non-interlaced 8-bit greyscale, truecolour, greyscale-alpha, and RGBA images.

Production is a pure in-memory transform. The package delegates raw RFC 1951
compression, counted decompression, and CRC-32 to `fsharp/zip`; it does not
duplicate DEFLATE or CRC logic and owns no filesystem, process, network,
environment, clock, entropy, FFI, or credential authority.

## Profile

- exact signature, chunk type, order, CRC-32, and end-of-file validation;
- exact zlib CM/CINFO/FCHECK, FDICT, Adler-32, DEFLATE consumption, and output
  length checks;
- None, Sub, Up, Average, and Paeth filters with the signed selection heuristic
  and normative Paeth tie order;
- colour types 0, 2, 4, and 6, suggested `PLTE`, `tRNS`, split consecutive
  `IDAT`, and unknown ancillary chunks;
- named rejection of APNG `acTL`, `fcTL`, and `fdAT` chunks;
- 16,384-pixel edge and 33,554,432-pixel product ceilings, with a caller limit
  that may only lower the product ceiling; and
- the closed 29-code `PngError` taxonomy from the shared 85-case corpus.

Palette images, alternate bit depths, Adam7 interlacing, preset dictionaries,
unknown critical chunks, and APNG animation are refused with stable errors.

## Usage

```fsharp
open CodingAdventures.ImageCodecPng.FSharp
open CodingAdventures.PixelContainer

let pixels = PixelContainer(2, 1)
pixels.SetPixel(0, 0, 255uy, 0uy, 0uy, 255uy)
pixels.SetPixel(1, 0, 0uy, 0uy, 255uy, 255uy)

let encoded = Png.encodePng pixels
let decoded = Png.decodePng encoded (Some 4096.0)
let codec: IImageCodec = PngCodec()
```

## Portable conformance

The tests consume every case in
`code/specs/fixtures/image-codec-png-v1/cases.json` through the public API.
JSON can express fractional dimensions and malformed RGBA lengths that the
typed `PixelContainer` constructor cannot represent, so the fixture adapter
rejects those values before conversion instead of truncating or repairing
them. Representable encode cases use `Png.encodePng`; .NET's independent
`ZLibStream` inflates their IDAT stream and verifies every pinned row filter.

Run the package front door with:

```sh
sh BUILD
```

It runs the public tests and enforces at least 90% line coverage.
