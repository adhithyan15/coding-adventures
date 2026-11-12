# Changelog

## [Unreleased]

### Changed

- `BUILD_windows` now runs `cabal test all` instead of skipping with "haskell support
  not enabled in windows CI yet", so this package is built and tested on Windows.

### Security

- No code change: block count is validated against the payload and the declared length must be satisfiable. NOT RUN locally (toolchain unavailable in the review environment).
- Added test/HostileHeadersSpec.hs (hspec, registered in the cabal test-suite and Spec.hs): hostile headers must be rejected promptly, plus round trips over every byte string of length <= 2 and a seeded pseudo-random sample. NOT RUN locally (toolchain unavailable in the review environment).

## Unreleased

- The Darwin BUILD now runs Cabal with the Command Line Tools developer dir
  (`DEVELOPER_DIR=/Library/Developer/CommandLineTools`), so GHC 9.4.8's CLT SDK
  sysroot gets the matching linker on macOS CI with Command Line Tools 27.

## 0.1.0 - 2026-07-13

- Add CMP02 LZSS literal and backreference tokenisation for strict bytes.
- Add overlap-safe decoding and configurable window and match thresholds.
- Add flag-block serialisation with big-endian length and block-count fields.
- Reject malformed parameters, tokens, headers, blocks, and trailing data.
- Add spec-vector, round-trip, parameter, wire-format, and behaviour tests.
