# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-18

### Added

- Pure C# LZ77 encoder, decoder, and teaching-oriented token serialisation helpers
- Overlap-safe decoding, configurable window and match limits, and one-shot `Compress` / `Decompress` helpers
- xUnit coverage for spec vectors, round trips, overlap handling, parameter constraints, and serialisation
- BUILD scripts that isolate `.NET` artifacts and first-run state for Linux and Windows CI
