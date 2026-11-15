# coding_adventures_image_codec_png

`coding_adventures_image_codec_png` provides bounded, deterministic PNG encoding and decoding for Elixir conforming to IC18 and RFC 2083.

## Architectural Reuse

In accordance with IC18 architectural principles, this package **does not duplicate DEFLATE or CRC-32**:
- CRC-32 chunk integrity uses `CodingAdventures.Zip.crc32/2`.
- Compressed IDAT decompression uses `CodingAdventures.Zip.raw_inflate_counted/2`, enforcing exact output byte limits.
- Compressed IDAT compression uses `CodingAdventures.Zip.raw_deflate/1`.
- Image buffers interface directly with `%CodingAdventures.PixelContainer{}`.
- Zero third-party Hex dependencies; `:zlib` is strictly forbidden in production code and permitted only in tests as an independent foreign verification oracle.

## Clean Code & Standing Rule #20

All diagnostic messages, error text, and redaction placeholders (`<redacted>`, `<invalid>`) are isolated in `CodingAdventures.ImageCodecPng.Messages`. Zero double-quote string literals appear in operational logic. No raw input payload data, chunk byte values, or untrusted string fragments are ever interpolated into error messages or exception strings.

## Public API

```elixir
alias CodingAdventures.ImageCodecPng
alias CodingAdventures.PixelContainer

# Encode an RGBA8 PixelContainer to PNG bytes
container = PixelContainer.new(64, 64)
png_bytes = ImageCodecPng.encode_png(container)

# Decode PNG bytes to an RGBA8 PixelContainer
decoded = ImageCodecPng.decode_png(png_bytes)

# Decode with custom caller pixel limit (max 33,554,432)
bounded = ImageCodecPng.decode_png(png_bytes, max_pixels: 100_000)

# Standalone RFC 1950 Adler-32 checksum calculation
checksum = ImageCodecPng.adler32(raw_bytes)

# ImageCodec behaviour
ImageCodecPng.mime_type() # => "image/png"
{:ok, container} = ImageCodecPng.decode(png_bytes)
```

## 6-Class PNG CVE Resilience Showcase

PNG decoders run on untrusted inputs from the web. This package explicitly neutralizes classic classes of PNG vulnerabilities:

1. **Integer & Dimension Overflow Bombs** (CVE-2004-0597, CVE-2019-7317):
   Decoded dimensions are bounded by `max_dimension = 16_384` and total pixel count is bounded by `max_pixels = 33_554_432` (32 mebipixels). Callers may lower the limit, but cannot raise or fractionalize it.

2. **Decompression & Zip Bombs** (CVE-2002-1363, CVE-2023-4863):
   Hostile IDAT streams can advertise 1032:1 DEFLATE compression ratios. Decompression is capped strictly at `height * (width * channels + 1)` via `Zip.raw_inflate_counted`. If output exceeds the promised length, decompression halts immediately with `inflated-length-mismatch`.

3. **Chunk-Length Truncation & Pre-Allocation OOM** (CVE-2018-14550):
   Declared chunk lengths are checked against the remaining file buffer *before* any buffer slicing or allocation. A declared length exceeding remaining file bytes is rejected as `truncated-chunk`.

4. **Paeth / Filter Arithmetic & Signedness Traps** (CVE-2015-8126):
   Predictor arithmetic is computed strictly with 8-bit modular arithmetic (`&&& 0xFF`). The normative Paeth tie-breaking sequence (`pa <= pb && pa <= pc` -> `a`, `pb <= pc` -> `b`, else `c`) prevents off-by-one and signedness deviations. Filter IDs > 4 are rejected as `invalid-filter`.

5. **Checksum & Integrity Tampering, IDAT Cavities** (CVE-2010-1205):
   Every chunk's CRC-32 is checked prior to parsing its payload. The uncompressed scanline Adler-32 is verified against the zlib trailer. Any mismatch raises `chunk-crc-mismatch` or `adler-mismatch`. Furthermore, any unconsumed bytes between DEFLATE termination and Adler-32 trigger `idat-cavity`.

6. **Chunk Ordering, Duplication & APNG Refusal** (CVE-2011-3045, CVE-2021-30860):
   Framing requires exactly one `IHDR` first, consecutive `IDAT` chunks, and an empty `IEND` terminating the stream. APNG chunks (`acTL`, `fcTL`, `fdAT`) are rejected as `unsupported-feature` to prevent hidden animation frames.
