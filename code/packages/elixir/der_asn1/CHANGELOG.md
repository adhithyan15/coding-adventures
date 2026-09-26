# Changelog

## [0.1.0] - 2026-09-26

### Added

- Bounded typed DER decoding for the portable ASN.1 value vocabulary.
- Sealed immutable decoder, element, and cursor values with exact-owner shared
  budgets and transactional failures.
- Exact unsigned-64 INTEGER and OBJECT IDENTIFIER validation using BEAM
  arbitrary-precision integers with explicit u64 bounds.
- The complete 122-case neutral suite, all 46 delegated DER TLV cases, native
  forgery, owner, limit, tag, equality, transactionality, and redaction tests,
  plus a 100 percent line-coverage gate.
- Unix and Windows build fronts and an empty capability manifest.
