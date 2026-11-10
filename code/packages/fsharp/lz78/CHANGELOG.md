# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Security

- Hardening (CWE-789): the declared length is clamped instead of raw-cast (>= 2^31 went negative = 'no truncation'); Decode stops once the declared length is reached (quadratic expansion before truncation). Token count was already capped. Not run locally.
- Added HostileHeadersTests (xunit): hostile headers with a wall-clock bound, plus round trips over every byte string of length <= 2 and a seeded random sample. NOT RUN locally (no dotnet SDK in the review environment); written to the existing test project conventions.

## [0.1.0] - 2026-04-18

### Added

- Pure F# LZ78 encoder, decoder, and CMP01 teaching-format serialisation helpers
- Dictionary-growth, flush-token, and bounded-deserialisation behavior for explicit-dictionary compression
- xUnit coverage for spec vectors, binary round trips, capped dictionary size, and serialisation symmetry
- BUILD scripts that isolate `.NET` artifacts and first-run state for Linux and Windows CI
- Deserialisation now caps token counts against the available payload so crafted headers cannot force oversized decode loops
