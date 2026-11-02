# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

- `BUILD` runs Cabal with `DEVELOPER_DIR=/Library/Developer/CommandLineTools` on Darwin so GHC 9.4.8 links with the Command Line Tools linker that accepts the CLT SDK's `arm64e.x1` stubs; other platforms are unchanged.

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
