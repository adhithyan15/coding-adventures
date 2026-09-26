# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-09-20

### Added

- Implemented the bounded typed DER decoder with immutable validated wrappers,
  shared depth and element budgets, transactional cursors, canonical primitive
  checks, unsigned-64-safe OID parsing, and payload-blind errors.
- Added package-native execution of all 122 neutral DER ASN.1 cases and all 46
  referenced DER TLV cases with line and branch coverage gates.
- Added StandardRB Unix and Windows build fronts plus an empty capability
  manifest.
