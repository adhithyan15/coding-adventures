# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-18

### Added

- Pure C# LZ78 encoder, decoder, and CMP01 teaching-format serialisation helpers
- Dictionary-growth, flush-token, and bounded-deserialisation behavior for explicit-dictionary compression
- xUnit coverage for spec vectors, binary round trips, capped dictionary size, and serialisation symmetry
- BUILD scripts that isolate `.NET` artifacts and first-run state for Linux and Windows CI
