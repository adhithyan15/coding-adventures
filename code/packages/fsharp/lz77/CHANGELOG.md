# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Security

- CWE-789/CWE-400: DeserialiseTokens looped 'for index in 0u .. count - 1u' over the untrusted header count (up to ~4.3 billion iterations, with int index * 4 overflowing into negative spans); the count is now clamped to the complete tokens present. Not run locally.
- Added HostileHeadersTests (xunit): hostile headers with a wall-clock bound, plus round trips over every byte string of length <= 2 and a seeded random sample. NOT RUN locally (no dotnet SDK in the review environment); written to the existing test project conventions.

## [0.1.0] - 2026-04-18

### Added

- Pure F# LZ77 encoder, decoder, and teaching-oriented token serialisation helpers
- Overlap-safe decoding, configurable window and match limits, and one-shot `Compress` / `Decompress` helpers
- xUnit coverage for spec vectors, round trips, overlap handling, parameter constraints, and serialisation
- BUILD scripts that isolate `.NET` artifacts and first-run state for Linux and Windows CI
