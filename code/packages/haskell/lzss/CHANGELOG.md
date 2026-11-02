# Changelog

## Unreleased

- `BUILD` runs Cabal with `DEVELOPER_DIR=/Library/Developer/CommandLineTools` on Darwin so GHC 9.4.8 links with the Command Line Tools linker that accepts the CLT SDK's `arm64e.x1` stubs; other platforms are unchanged.

## 0.1.0 - 2026-07-13

- Add CMP02 LZSS literal and backreference tokenisation for strict bytes.
- Add overlap-safe decoding and configurable window and match thresholds.
- Add flag-block serialisation with big-endian length and block-count fields.
- Reject malformed parameters, tokens, headers, blocks, and trailing data.
- Add spec-vector, round-trip, parameter, wire-format, and behaviour tests.
