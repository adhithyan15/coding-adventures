# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Security

- Hardening (CWE-789): DeserialiseTokens clamps the token count to the data present and clamps (rather than raw-casts) the declared length, since a length >= 2^31 went negative and Decode treated negative as 'no truncation'. Decode stops once the declared length is reached (quadratic expansion before truncation). Not run locally.
- Added HostileHeadersTests (xunit): hostile headers with a wall-clock bound, plus round trips over every byte string of length <= 2 and a seeded random sample. NOT RUN locally (no dotnet SDK in the review environment); written to the existing test project conventions.

## [0.1.0] - 2026-04-18

### Added

- Pure C# LZ78 encoder, decoder, and CMP01 teaching-format serialisation helpers
- Dictionary-growth, flush-token, and bounded-deserialisation behavior for explicit-dictionary compression
- xUnit coverage for spec vectors, binary round trips, capped dictionary size, and serialisation symmetry
- BUILD scripts that isolate `.NET` artifacts and first-run state for Linux and Windows CI
