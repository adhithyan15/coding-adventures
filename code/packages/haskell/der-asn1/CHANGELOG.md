# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Fixed

- The Darwin BUILD now runs Cabal with the Command Line Tools developer dir
  (`DEVELOPER_DIR=/Library/Developer/CommandLineTools`), so GHC 9.4.8's CLT SDK
  sysroot gets the matching linker on macOS CI with Command Line Tools 27.

## [0.1.0] - 2026-09-26

### Added

- Implemented the complete `der-asn1-v1` portable fixture contract.
- Added concurrency-safe exact-owner shared work accounting and transactional
  cursor reads.
- Added bounded canonical primitive, constructed, explicit, and OID decoding.
- Added Unix and Windows build fronts, warning-clean builds, and HPC coverage.
