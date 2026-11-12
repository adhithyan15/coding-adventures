# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Changed

- `BUILD_windows` now runs `cabal test` instead of skipping with "haskell support
  not enabled in windows CI yet", so this package is built and tested on Windows.

### Fixed

- The Darwin BUILD now runs Cabal with the Command Line Tools developer dir
  (`DEVELOPER_DIR=/Library/Developer/CommandLineTools`), so GHC 9.4.8's CLT SDK
  sysroot gets the matching linker on macOS CI with Command Line Tools 27.

## [0.1.0] - 2026-07-29

### Added

- Add ordered HTTP headers with ASCII case-insensitive first-value lookup.
- Add bounded HTTP versions, body framing hints, and semantic request/response
  head records with content helpers.
- Reject oversized decimal version and content-length values with a bounded
  fold that does not allocate attacker-sized arbitrary-precision integers.
- Add raw request-target and query helpers that preserve percent-encoded text,
  duplicate parameters, flags, and fragments.
- Add small path-only route patterns with ordered named captures.
- Add a 19-example Hspec suite covering valid, malformed, boundary, delegation,
  query, path, and routing behavior.
