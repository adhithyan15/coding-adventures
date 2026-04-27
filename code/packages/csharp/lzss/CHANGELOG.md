# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-18

### Added

- Pure C# LZSS encoder, decoder, and CMP02 flag-block serialisation helpers
- Overlap-safe match decoding and bounded block deserialisation for the LZ77-with-flags variant
- xUnit coverage for spec vectors, wire-format symmetry, binary round trips, and compression behavior
- BUILD scripts that isolate `.NET` artifacts and first-run state for Linux and Windows CI
