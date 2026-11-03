# Changelog

## Unreleased

- The Darwin BUILD now runs Cabal with the Command Line Tools developer dir
  (`DEVELOPER_DIR=/Library/Developer/CommandLineTools`), so GHC 9.4.8's CLT SDK
  sysroot gets the matching linker on macOS CI with Command Line Tools 27.

## 0.1.0 - 2026-07-13

- Add CMP03 LZW coding with a pre-seeded 256-byte dictionary.
- Add the CLEAR, STOP, and self-referential tricky-token behaviours.
- Add variable-width 9-to-16-bit LSB-first code packing.
- Reject malformed headers, code streams, padding, and truncated payloads.
- Add reference-vector, round-trip, wire-format, growth, and validation tests.
