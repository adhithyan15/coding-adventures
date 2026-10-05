# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

### Added

- Add a bounded `barcode-layout-1d-v1` facade for binary and width expansion,
  symbol geometry, and rectangle-only scene projection. The legacy helpers
  retain their existing behavior.
- Run the shared 56-case neutral corpus in native ExUnit, including stable
  error and text-before-resolver assertions.

## [0.1.0] - 2026-04-12

### Added

- Initial 1D barcode layout primitives and PaintScene translation
