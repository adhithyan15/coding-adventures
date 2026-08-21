# image-codec-png (Go)

`image-codec-png` is the Go implementation of the bounded IC18 portable PNG
profile. It encodes RGBA8 `PixelContainer` values and decodes non-interlaced,
8-bit greyscale, truecolour, greyscale-alpha, and RGBA PNGs.

The production package is a pure in-memory byte transform. CRC-32 and raw
RFC 1951 compression come from the repository's `go/zip` package; this module
does not duplicate either codec and does not call Go's `image/png` or
`compress/zlib` packages. Those standard-library packages appear only in tests
as independent interoperability oracles.

## Supported profile

- exact PNG signature, chunk type, ordering, CRC-32, and end-of-file checks;
- stored, fixed-Huffman, and dynamic-Huffman RFC 1951 decoding;
- exact DEFLATE consumption plus RFC 1950 CM/CINFO/FCHECK, FDICT, and Adler-32;
- filters None, Sub, Up, Average, and Paeth, including normative tie handling;
- colour types 0, 2, 4, and 6 at 8-bit depth;
- suggested `PLTE`, greyscale/truecolour `tRNS`, split consecutive `IDAT`, and
  unknown ancillary chunks;
- 16,384-pixel edge and 33,554,432-pixel product ceilings, with a caller limit
  that may only lower the product ceiling;
- the closed 29-code `PngError` taxonomy from the shared 85-case corpus.

Palette images, Adam7 interlacing, alternate bit depths, preset zlib
dictionaries, unknown critical chunks, and APNG control/data chunks are refused
with stable errors.

## Usage

```go
import (
    png "github.com/adhithyan15/coding-adventures/code/packages/go/image-codec-png"
    pixel "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"
)

image := pixel.New(2, 1)
pixel.SetPixel(image, 0, 0, 255, 0, 0, 255)
pixel.SetPixel(image, 1, 0, 0, 0, 255, 255)

encoded, err := png.EncodePNG(image)
if err != nil {
    panic(err)
}

maxPixels := float64(4096)
decoded, err := png.DecodePNG(encoded, png.DecodeOptions{MaxPixels: &maxPixels})
if err != nil {
    panic(err)
}
_ = decoded
```

`PngCodec` implements `pixelcontainer.ImageCodec`. Because that historical
interface cannot return an encode error, its `Encode` method panics with the
same typed `*PngError` that `EncodePNG` returns. New code that handles untrusted
or dynamically assembled containers should call `EncodePNG` directly.

## Portable conformance

The test suite consumes every case in
`code/specs/fixtures/image-codec-png-v1/cases.json` through the public API when
the value can inhabit Go's typed `PixelContainer`. JSON can express a fractional
width, while `PixelContainer.Width` is a `uint32`; the fixture adapter therefore
rejects that one invalid value before conversion instead of truncating it. All
representable encode cases use `EncodePNG`, are independently inflated and
filter-inspected with `compress/zlib`, and are decoded by `image/png`.

Run the full package gates with:

```sh
go test ./... -race -cover
go vet ./...
go build -trimpath ./...
```

## Dependency graph

```text
pixel-container ─┐
                 ├─ image-codec-png
zip ─ lzss ──────┘
```

The later `go/paint-codec-png` reconciliation is intentionally separate; that
adapter still delegates to the standard library until its own queued migration.
