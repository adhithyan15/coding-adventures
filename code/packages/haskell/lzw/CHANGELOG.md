# Changelog

## Unreleased

- `BUILD` runs Cabal with `DEVELOPER_DIR=/Library/Developer/CommandLineTools` on Darwin so GHC 9.4.8 links with the Command Line Tools linker that accepts the CLT SDK's `arm64e.x1` stubs; other platforms are unchanged.

## 0.1.0 - 2026-07-13

- Add CMP03 LZW coding with a pre-seeded 256-byte dictionary.
- Add the CLEAR, STOP, and self-referential tricky-token behaviours.
- Add variable-width 9-to-16-bit LSB-first code packing.
- Reject malformed headers, code streams, padding, and truncated payloads.
- Add reference-vector, round-trip, wire-format, growth, and validation tests.
