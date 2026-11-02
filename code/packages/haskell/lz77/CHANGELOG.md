# Changelog

## Unreleased

- `BUILD` runs Cabal with `DEVELOPER_DIR=/Library/Developer/CommandLineTools` on Darwin so GHC 9.4.8 links with the Command Line Tools linker that accepts the CLT SDK's `arm64e.x1` stubs; other platforms are unchanged.

## 0.1.0 - 2026-07-13

- Add CMP00 LZ77 encoding and overlapping-match decoding for strict bytes.
- Add configurable window and match thresholds with wire-width validation.
- Add the fixed-width big-endian token serialisation format.
- Reject malformed tokens, offsets, headers, and truncated token streams.
- Add spec-vector, round-trip, parameter, wire-format, and behaviour tests.
