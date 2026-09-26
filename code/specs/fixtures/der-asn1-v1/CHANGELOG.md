# Changelog

## Unreleased

- Added the v1 typed DER value schema and corpus.
- Reused all 46 exact DER TLV framing cases by stable reference.
- Added closed semantic results, decimal-string integer and OID projections,
  shared-budget cursor scripts, explicit offset domains, and redacted errors.
- Expanded the closed corpus to 122 cases with valid OID arcs at `2^63` and
  `2^64 - 1`, implicit-OID rejection and budget cases, nested exact and
  over-limit depth coverage, shared nested element budgets, and
  explicit-wrapper class, construction, and tag-number rejection cases.
