# image-codec-png (Java)

Native Java implementation of the bounded IC18 PNG profile. It encodes RGBA8
`PixelContainer` values as non-interlaced colour-type-6 PNGs and decodes
non-interlaced 8-bit colour types 0, 2, 4, and 6, including suggested `PLTE`
and grayscale/truecolour `tRNS` transparency.

The package delegates raw RFC 1951 compression, counted decompression, and
CRC-32 to the sibling `zip` package. Production does not use ImageIO,
`java.util.zip`, files, processes, environment state, networking, reflection,
or native code. Those JDK facilities appear only in tests as independent
interoperability oracles.

## API

- `Png.encodePng(PixelContainer)` returns a complete PNG byte array.
- `Png.decodePng(byte[])` decodes with the default 32-mebipixel ceiling.
- `Png.decodePng(byte[], double)` accepts a positive integral lower ceiling.
- `Png.adler32(byte[])` exposes the RFC 1950 checksum.
- `PngCodec` implements the shared `ImageCodec` contract with MIME type
  `image/png`.
- `PngError.code()` is one of the 29 ordered identifiers in
  `Png.ERROR_CODES`; its message is exactly that payload-blind code.

`Png.MAX_DIMENSION` is 16,384 and `Png.DEFAULT_MAX_PIXELS` is 33,554,432.
Both edge and product checks happen before filtered or RGBA allocation. Decode
also caps raw inflation at the exact scanline size promised by IHDR and requires
exact DEFLATE byte consumption, Adler-32, chunk CRCs, and legal chunk order.

## Validation

The test suite consumes all 85 cases in the language-neutral
`image-codec-png-v1` corpus through public APIs. Encoder output is independently
decoded with ImageIO and its zlib stream with the JDK inflater; filter choices,
all 29 errors, APNG refusal precedence, resource limits, and malformed explicit
PixelContainer buffers are load-bearing. JaCoCo enforces at least 90% line
coverage.

```bash
gradle test jacocoTestReport jacocoTestCoverageVerification
```
