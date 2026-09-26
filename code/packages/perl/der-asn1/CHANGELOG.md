# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-09-20

### Added

- Implemented the complete 122-case `der-asn1-v1` portable contract and all 46
  delegated DER TLV fixture references.
- Added canonical primitive, container, explicit-tag, exact `u64` OID, shared
  budget, transactional cursor, local-offset, and redacted-error behavior.
- Added private field-hash provenance for validated wrappers and exact decoder
  ownership, including adversarial native tests.
- Added Unix and Windows build fronts, capability metadata, packaging metadata,
  and core-only test dependencies.
