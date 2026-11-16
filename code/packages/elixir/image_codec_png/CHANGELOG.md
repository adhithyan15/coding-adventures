# Changelog

All notable changes to `coding_adventures_image_codec_png` will be documented in this file.

## [0.1.0] - 2026-11-15

### Added
- Complete IC18 PNG encoder and decoder implementing the `CodingAdventures.ImageCodec` behaviour.
- Architectural reuse of `CodingAdventures.Zip` for DEFLATE compression, bounded counted decompression, and CRC-32 chunk validation.
- Zero host/stdlib crypto imports in operational logic; zero Hex dependencies.
- Strict Standing Rule #20 compliance with payload-blind error diagnostics centralized in `CodingAdventures.ImageCodecPng.Messages`.
- 6-Class PNG CVE resilience showcase against decompression bombs, dimension overflows, chunk truncation, filter traps, and IDAT cavities.
- 10,000-iteration deterministic PRNG property and mutation fuzz testing suite with 98.62% code coverage.
