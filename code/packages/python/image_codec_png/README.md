# coding-adventures-image-codec-png

A pure Python implementation of the bounded PNG profile in
[`IC18`](../../../specs/IC18-image-codec-png.md). The package encodes RGBA8
`PixelContainer` values and decodes non-interlaced, 8-bit grayscale, RGB,
grayscale-alpha, and RGBA PNGs.

## API

```python
from image_codec_png import PngCodec, decode_png, encode_png
from pixel_container import PixelContainer

pixels = PixelContainer(1, 1, bytearray([255, 0, 0, 255]))
encoded = encode_png(pixels)
assert decode_png(encoded) == pixels
assert PngCodec().mime_type == "image/png"
```

The public surface also exports `adler32`, the immutable 29-code
`PNG_ERROR_CODES` taxonomy, `PngError.code`, `PNG_MAX_DIMENSION` (16,384), and
`PNG_MAX_PIXELS` (33,554,432). `decode_png(..., max_pixels=...)` and
`PngCodec(max_pixels=...)` accept only a positive exact `int` no larger than
the default. Python `bool` values are rejected rather than treated as integers.

## Profile and security boundary

- Encoding always writes deterministic 8-bit colour-type-6 PNG with one IDAT.
- Decoding supports colour types 0, 2, 4, and 6, suggested PLTE, tRNS for
  grayscale/RGB, split consecutive IDAT, and unknown ancillary chunks.
- Palette, Adam7, non-8-bit, APNG, unknown critical, malformed chunk, and
  non-exact zlib streams fail with stable payload-blind `PngError` codes.
- Edge and product limits are checked before multiplication-dependent
  allocation. Raw inflate is capped at the exact scanline size promised by
  IHDR, and exact DEFLATE consumption, Adler-32, CRC-32, and filter bytes are
  all verified before RGBA allocation.
- Production depends only on repository PixelContainer and ZIP. ZIP owns raw
  RFC 1951 and CRC-32; this package does not duplicate them.

PixelContainer and counted inflate both use compact `bytearray`/`bytes`
storage. ZIP raw DEFLATE also switches large input to bounded blocks with
constant-size match state or stored framing. The published 32-mebipixel ceiling
therefore does not degrade into boxed-byte multi-gigabyte amplification during
either decode or encode.

## Conformance

The test suite consumes all 85 cases in the language-neutral
[`image-codec-png-v1`](../../../specs/fixtures/image-codec-png-v1/README.md)
corpus through public APIs. Python's standard-library zlib independently
inflates encoder output and test-only Pillow accepts the complete PNG and
recovers exact RGBA bytes. Ruff, formatting, strict MyPy, branch coverage of at
least 90%, capability validation, and the repository build-tool closure are
part of the package front door.

The production capability manifest is intentionally empty. Filesystem access
used to load fixtures and Pillow's native image tooling exist only in tests.
