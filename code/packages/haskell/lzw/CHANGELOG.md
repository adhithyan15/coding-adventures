# Changelog

## [Unreleased]

### Changed

- `BUILD_windows` now runs `cabal test all` instead of skipping with "haskell support
  not enabled in windows CI yet", so this package is built and tested on Windows.

### Security

- No code change: no allocation is sized from the header; the decoded length is checked against it. NOT RUN locally (toolchain unavailable in the review environment).
- Added test/HostileHeadersSpec.hs (hspec, registered in the cabal test-suite and Spec.hs): hostile headers must be rejected promptly, plus round trips over every byte string of length <= 2 and a seeded pseudo-random sample. NOT RUN locally (toolchain unavailable in the review environment).

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
