# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-09-26

### Added

- Bounded typed DER decoding for the portable ASN.1 value vocabulary.
- Private validated wrappers, exact decoder ownership, shared transactional
  budgets, defensive projections, and payload-blind errors.
- Exact unsigned-64 INTEGER and OBJECT IDENTIFIER projections using two 32-bit
  limbs, including the full portable boundary above Lua's signed integer max.
- The complete 122-case neutral suite, all 46 delegated DER TLV cases, native
  forgery, owner, limit, tag, equality, transactionality, and redaction checks,
  plus a 95 percent production line-coverage gate.
- Unix and Windows build fronts, LuaRocks metadata, and an empty capability
  manifest.
