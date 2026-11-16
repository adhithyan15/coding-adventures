# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-15

### Added
- Initial implementation of IC18 PNG encoder and decoder in Ruby (`CodingAdventures::ImageCodecPng`).
- Full support for RFC 2083 8-bit truecolour (RGBA, RGB) and greyscale (with and without alpha) image decoding.
- Lossless RGBA8 non-interlaced single-IDAT encoding with RFC 2083 adaptive row filtering.
- Zero-copy integration with `CodingAdventures::Zip` (`raw_deflate`, `raw_inflate_counted`, `crc32`) and `CodingAdventures::PixelContainer`.
- RFC 1950 Adler-32 implementation (`CodingAdventures::ImageCodecPng.adler32`).
- Strict 7-class PNG CVE resilience defenses against decompression bombs, integer overflows, chunk truncation OOMs, and IDAT cavities.
- 100% payload-blind error diagnostics and message isolation adhering to Standing Rule #20.
- Exhaustive test suite replaying all 85 fixture vectors, property-based round trips, and 10,000-iteration adversarial fuzzing.
