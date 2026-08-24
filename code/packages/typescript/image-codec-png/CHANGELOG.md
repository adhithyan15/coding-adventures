# Changelog — @coding-adventures/image-codec-png

## 0.2.1 — 2026-08-20

### Changed

- Refuse APNG `acTL`, `fcTL`, and `fdAT` chunks as `unsupported-feature`
  after normal chunk-type and CRC validation rather than skipping them as
  unknown ancillary data.
- Consume the expanded 85-case neutral corpus, including all three valid-CRC
  APNG rejection vectors, while keeping the 29-code taxonomy unchanged.

## 0.2.0 — 2026-08-20

### Added

- `PngError`, `PngErrorCode`, `PNG_ERROR_CODES`, `PNG_MAX_DIMENSION`, and
  `PNG_MAX_PIXELS` as the stable public IC18 failure and resource surface.
- Full consumption of the 82-case language-neutral `image-codec-png-v1`
  corpus, including exact portable errors and foreign `pngjs` decoding of
  encoder output.
- Explicit empty production capability metadata.

### Changed

- `maxPixels` is now a positive safe integer no larger than 32 mebipixels, so a
  caller can only lower the allocation ceiling.
- Zlib headers now reject `CINFO > 7`, invalid chunk-type bytes and a lowercase
  reserved third type letter are refused, and ZIP-owned inflater failures are
  mapped to the PNG error taxonomy.
- Suggested `PLTE` acceptance for truecolour images and exact `tRNS`
  transparency for greyscale/truecolour inputs, with closed malformed ordering,
  length, duplication, and sample-range failures.
- Normative encoder filter-choice pins plus explicit Paeth predictor branch and
  tie vectors.

### Security

- Stable error identifiers no longer require consumers to parse messages.
- The independent corpus closes exact chunk/IDAT boundaries, CRC and Adler,
  stored/fixed/dynamic DEFLATE, filter types, unsupported features, pixel and
  dimension limits, malformed inflation, and covert IDAT cavities.

## 0.1.0 — 2026-08-12

Initial release. Implements [`IC18`](../../../specs/IC18-image-codec-png.md).

### Added

- `encodePng(pixels)` — `PixelContainer` → 8-bit RGBA PNG (colour type 6), one
  `IDAT`, non-interlaced. Type 6 because it is exactly what a `PixelContainer`
  holds, so the round trip is lossless by construction rather than by luck.
- `decodePng(bytes)` — reads 8-bit colour types 0, 2, 4 and 6, non-interlaced,
  any number of `IDAT` chunks, skipping unknown ancillary chunks and refusing
  unknown critical ones as RFC 2083 requires.
- `PngCodec` implementing `ImageCodec`, mime type `image/png`.
- `adler32(data)` — the RFC 1950 checksum, exported because it is testable on
  its own and because nothing else in the repo had one.
- All five scanline filters (None, Sub, Up, Average, Paeth) on both sides, with
  per-row selection by the PNG spec's own minimum-sum-of-signed-bytes heuristic.

### Depends on

- `@coding-adventures/pixel-container` (IC00) for the RGBA8 buffer.
- `@coding-adventures/zip` (CMP09) for `rawDeflate`, `rawInflate` and `crc32`.
  DEFLATE is the compressor inside zlib, gzip and PNG's `IDAT`, and PNG chunks
  use ZIP's CRC-32 polynomial — a second copy of either would be a second place
  for the same class of bit-packing bug to hide.

### Refused by name, not half-supported

Palette images (colour type 3), bit depths other than 8, Adam7 interlacing, and
APNG animation. A decoder that silently mis-reads a palette image is worse than
one that says it cannot read it.

### Security

- Malformed input always throws; never partial or approximate output.
- Every chunk CRC-32 and the trailing Adler-32 are verified.
- A chunk's declared length is checked against the file size **before** any
  arithmetic uses it.
- Each edge is capped at 16,384 pixels (matching IC01) **and the total pixel
  count at 32 mebipixels**, configurable via `maxPixels`. An edge cap alone is
  not enough: 16384 × 16384 passes it and is 268 million pixels, about 3 GiB of
  peak allocation for roughly a megabyte of input. BMP survives on an edge cap
  because its pixels must be present in the file; PNG amplifies.
- `IHDR` must be first, `IEND` must be empty and last, `IDAT` chunks must be
  consecutive, and the DEFLATE stream must end exactly where the Adler-32
  begins. Each violation
  yields a file that decodes to the right image while carrying bytes the image
  does not need — the last being the `IDAT` cavity, since DEFLATE announces its
  own end and a decoder asking only for pixels never inspects the remainder.
- Inflation is capped at exactly the size `IHDR` promises, so a bomb inside
  `IDAT` is stopped at the only size the image could possibly need. DEFLATE's
  expansion ratio reaches 1032:1.

### Tests

58 tests. Round-trip tests only prove the encoder and decoder agree with each
other, so the suite also tests against foreign implementations:

- the encoder's `IDAT` is inflated with **Node's zlib** and its scanline count
  and filter bytes checked;
- the decoder is fed PNGs assembled **by hand from RFC 2083** and compressed by
  Node's zlib — one image exercising all five filter types, plus one per
  supported colour type, plus a split-`IDAT` case;
- `adler32` is checked against the RFC's worked example (`"Wikipedia"` →
  `0x11E60398`) and against the trailer zlib itself writes, across the 5552-byte
  chunking boundary.

The written file was confirmed readable by `file`, macOS `sips`, and Python's
`zlib`. A 200×120 test image compresses 96,000 bytes to 2,123 — 45:1 — choosing
Paeth for 101 rows, Up for 14 and Sub for 5.
