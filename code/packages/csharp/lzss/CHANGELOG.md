# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Security

- Hardening (CWE-789): the declared length is clamped instead of raw-cast (>= 2^31 went negative = 'no truncation'); Decode stops once the declared length is reached. block_count was already capped. Not run locally.
- Added HostileHeadersTests (xunit): hostile headers with a wall-clock bound, plus round trips over every byte string of length <= 2 and a seeded random sample. NOT RUN locally (no dotnet SDK in the review environment); written to the existing test project conventions.

## [0.1.0] - 2026-04-18

### Added

- Pure C# LZSS encoder, decoder, and CMP02 flag-block serialisation helpers
- Overlap-safe match decoding and bounded block deserialisation for the LZ77-with-flags variant
- xUnit coverage for spec vectors, wire-format symmetry, binary round trips, and compression behavior
- BUILD scripts that isolate `.NET` artifacts and first-run state for Linux and Windows CI
