# ImageCodecPNG (Swift)

`ImageCodecPNG` is the Swift implementation of IC18. It encodes RGBA8
`PixelContainer` values and decodes the bounded, portable PNG profile without
filesystem, network, process, environment, or native-code authority.

## Supported profile

- Encodes 8-bit, non-interlaced colour type 6 with deterministic best-of-five
  row filtering.
- Decodes 8-bit colour types 0, 2, 4, and 6.
- Accepts split consecutive `IDAT`, suggested `PLTE`, truecolour/greyscale
  `tRNS`, and unknown non-semantic ancillary chunks.
- Rejects palette images, Adam7, non-8-bit samples, unknown critical chunks,
  and APNG `acTL`, `fcTL`, and `fdAT` chunks with stable `PngError` codes.

The implementation delegates raw RFC 1951 compression, counted inflation, and
CRC-32 to the repository `Zip` package. It does not carry a second DEFLATE or
CRC implementation.

## API

```swift
import ImageCodecPNG
import PixelContainer

var pixels = PixelContainer(width: 1, height: 1)
pixels.data = [255, 0, 0, 255]

let encoded = try encodePng(pixels)
let decoded = try decodePng(encoded)
let lowered = try decodePng(encoded, maxPixels: 1)

let codec = PngCodec()
precondition(codec.mimeType == "image/png")
```

Public limits are `pngMaxDimension` (16,384) and `pngDefaultMaxPixels`
(33,554,432). `pngErrorCodes` exposes the exact ordered 29-code IC18 taxonomy.
`PngError.description` is only its payload-blind code.

`encodePng(_:)` is throwing because `PixelContainer.data` is publicly mutable
and can disagree with its dimensions. IC00's historical `ImageCodec.encode`
requirement is nonthrowing, so `PngCodec.encode` is a valid-container
compatibility witness and fails fast on malformed mutable state. Call the
throwing helper whenever invalid containers must be handled as `PngError`.

## Security boundary

- Width and height are limited to 16,384 before integer conversion or product
  arithmetic; the product is capped at 32 mebipixels.
- Chunk lengths are bounded against remaining input before offsets, slicing, or
  allocation.
- The ZIP inflater is capped at the exact filtered size promised by `IHDR`, and
  the decoder requires exact output length and exact compressed-byte use.
- Every chunk CRC, the zlib header, Adler-32, filter byte, ordering rule, and
  required chunk is validated before allocating the RGBA output.
- Errors expose stable codes and never include input bytes or attacker-provided
  chunk payloads.

Production imports only `PixelContainer` and `Zip`. Foundation process APIs,
Python zlib/full-PNG decoding, and the platform ImageIO/WIC decoder checks are
test-only interoperability oracles, so the production capability manifest is
empty.

## Validation

```bash
swift format lint Sources/ImageCodecPNG/PNG.swift Tests/ImageCodecPNGTests/*.swift
swift test --enable-code-coverage
swift build -c release -Xswiftc -warnings-as-errors
```

The portable suite pins schema/profile/limits/error ordering and consumes all
85 shared fixtures. Encoder output is independently decompressed and decoded by
a test-only Python implementation, then accepted by a real platform image
decoder (ImageIO on macOS and WIC on Windows), in addition to the Swift round
trip.
