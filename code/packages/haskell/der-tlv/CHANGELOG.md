# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

- `BUILD` runs Cabal with `DEVELOPER_DIR=/Library/Developer/CommandLineTools` on Darwin so GHC 9.4.8 links with the Command Line Tools linker that accepts the CLT SDK's `arm64e.x1` stubs; other platforms are unchanged.

## [0.1.0] - 2026-09-19

### Added

- Bounded DER tag and definite-length decoding with all 17 portable error
  identifiers and offsets.
- Exact decoding and transactional cursor APIs over strict `ByteString`
  slices.
- Language-neutral conformance coverage for all 54 shared fixtures, including
  hostile inputs, configured limits, and redacted failures.
- Warning-as-error Linux and Windows build fronts and truthful empty capability
  metadata.
