# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Fixed

- The Darwin BUILD now runs Cabal with the Command Line Tools developer dir
  (`DEVELOPER_DIR=/Library/Developer/CommandLineTools`), so GHC 9.4.8's CLT SDK
  sysroot gets the matching linker on macOS CI with Command Line Tools 27.

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
