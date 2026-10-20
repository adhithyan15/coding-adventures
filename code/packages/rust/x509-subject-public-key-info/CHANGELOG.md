# Changelog

## Unreleased

- Exposed the exact borrowed canonical SubjectPublicKeyInfo sequence contents
  for trust-anchor identity matching without reparsing.

## 0.1.0 — 2026-09-20

- Added allocation-free generic RFC 5280 `SubjectPublicKeyInfo` decoding.
- Composed validated borrowed algorithm identifiers and canonical BIT STRINGs.
- Added redacted local-offset errors and adversarial structure, algorithm,
  framing, BIT STRING, and shared-budget tests.
