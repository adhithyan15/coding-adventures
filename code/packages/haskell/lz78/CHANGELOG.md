# Changelog

## [Unreleased]

### Changed

- `BUILD_windows` now runs `cabal test all` instead of skipping with "haskell support
  not enabled in windows CI yet", so this package is built and tested on Windows.

### Security

- decode stops once the declared length is reached (a hostile chain of tokens previously expanded quadratically through list appends before the final trim). Header validation was already strict (exact wire length). NOT RUN locally (toolchain unavailable in the review environment).
- Added test/HostileHeadersSpec.hs (hspec, registered in the cabal test-suite and Spec.hs): hostile headers must be rejected promptly, plus round trips over every byte string of length <= 2 and a seeded pseudo-random sample. NOT RUN locally (toolchain unavailable in the review environment).

## Unreleased

- The Darwin BUILD now runs Cabal with the Command Line Tools developer dir
  (`DEVELOPER_DIR=/Library/Developer/CommandLineTools`), so GHC 9.4.8's CLT SDK
  sysroot gets the matching linker on macOS CI with Command Line Tools 27.

## 0.1.0

- Add CMP01 LZ78 token encoding and checked decoding.
- Add an immutable byte-trie cursor for stepwise dictionary construction.
- Add strict fixed-width wire serialization and one-shot compression helpers.
