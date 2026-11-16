# Changelog: image-codec-png

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-15

### Added
- Initial release of portable RFC 2083 PNG encoder and bounded decoder (`CodingAdventures.ImageCodecPng`) in Haskell (IC18).
- Pure computation over in-memory buffers with empty capability manifest (`required_capabilities.json`).
- Architectural reuse of repository dependencies: `PixelContainer` (IC00 universal RGBA8 pixel container and `ImageCodec` type class) and `Zip` (CMP09 CRC-32, raw RFC 1951 DEFLATE, and counted inflate).
- Closed 29-constructor `PngError` sum type with 100% payload-blind diagnostic message extraction in `Messages.hs` (Standing Rule #20).
- Deterministic colour-type-6 encoder evaluating all 5 scanline filters (None, Sub, Up, Average, Paeth) and selecting the optimal filter via sum of absolute differences with normative tie-breaking.
- Full support for 8-bit non-interlaced greyscale, truecolour, greyscale-alpha, and truecolour-alpha with transparent color keys (`tRNS`) and palette validation (`PLTE`).
- 7-class CVE resilience: Word32/64-bit integer overflow protection, decompression bomb limits, chunk-length pre-allocation bounds, unsigned modulo arithmetic, CRC-32/Adler-32/IDAT cavity verification, strict chunk ordering, and payload-blind exception handling.
- Conformance test suite verifying all 85 neutral test vectors in `cases.json`, Python foreign stdlib `zlib` cross-oracle verification on every encode, and 10,000-iteration deterministic PRNG mutation fuzz loop.
