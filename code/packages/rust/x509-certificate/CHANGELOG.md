# Changelog

## 0.1.1 — 2026-10-24

- Added a composable certificate-element entry point that consumes an
  enclosing decoder's existing DER depth, element, length, and OID budgets.
- Kept the standalone exact-document decoder and all certificate semantics
  unchanged.

## 0.1.0 — 2026-10-18

- Added allocation-free decoding for complete RFC 5280 `Certificate` and
  `TBSCertificate` structures.
- Added exact signed-byte retention, signature-algorithm consistency checks,
  a non-empty issuer requirement, canonical implicit unique identifiers,
  optional extensions, and version constraints.
- Added redacted local-offset errors and adversarial structure, ordering,
  version, signature, framing, and shared-budget tests.
