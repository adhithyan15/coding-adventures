# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Security

- Hardening (CWE-789): DeserialiseTokens clamps the untrusted header count to the complete tokens present instead of iterating over the claimed count (the old loop also overflowed (int)i * 4 for counts beyond ~536M). Not run locally.
- Added HostileHeadersTests (xunit): hostile headers with a wall-clock bound, plus round trips over every byte string of length <= 2 and a seeded random sample. NOT RUN locally (no dotnet SDK in the review environment); written to the existing test project conventions.

## [0.1.0] - 2026-04-18

### Added

- Pure C# LZ77 encoder, decoder, and teaching-oriented token serialisation helpers
- Overlap-safe decoding, configurable window and match limits, and one-shot `Compress` / `Decompress` helpers
- xUnit coverage for spec vectors, round trips, overlap handling, parameter constraints, and serialisation
- BUILD scripts that isolate `.NET` artifacts and first-run state for Linux and Windows CI
