# Changelog

All notable changes to this package are documented here.

## [0.1.0] - Unreleased

### Added (VOS-028b)

- The probe fixture: `insert8`, `insert10`, `pick8`; the Warrant document
  `code/specs/vos-probe-ready.warrant` with five exhaustive claims (binder products 256,
  65,536, 65,536, 262,144 and 393,216) and four examples; the generated harness; behaviour
  tests. Every negative control is a real one (`shift-mask`), so no control is a known failure.
