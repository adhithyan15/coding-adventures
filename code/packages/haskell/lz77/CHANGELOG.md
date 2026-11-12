# Changelog

## [Unreleased]

### Changed

- `BUILD_windows` now runs `cabal test all` instead of skipping with "haskell support
  not enabled in windows CI yet", so this package is built and tested on Windows.

### Security

- No code change: the header count is validated against the payload length before any token is parsed, and invalid offsets return Left. NOT RUN locally (toolchain unavailable in the review environment).
- Added test/HostileHeadersSpec.hs (hspec, registered in the cabal test-suite and Spec.hs): hostile headers must be rejected promptly, plus round trips over every byte string of length <= 2 and a seeded pseudo-random sample. NOT RUN locally (toolchain unavailable in the review environment).

## Unreleased

- The Darwin BUILD now runs Cabal with the Command Line Tools developer dir
  (`DEVELOPER_DIR=/Library/Developer/CommandLineTools`), so GHC 9.4.8's CLT SDK
  sysroot gets the matching linker on macOS CI with Command Line Tools 27.

## 0.1.0 - 2026-07-13

- Add CMP00 LZ77 encoding and overlapping-match decoding for strict bytes.
- Add configurable window and match thresholds with wire-width validation.
- Add the fixed-width big-endian token serialisation format.
- Reject malformed tokens, offsets, headers, and truncated token streams.
- Add spec-vector, round-trip, parameter, wire-format, and behaviour tests.
