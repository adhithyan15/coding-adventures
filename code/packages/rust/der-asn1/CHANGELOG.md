# Changelog

## 0.1.0 — 2026-09-19

- Added allocation-free typed DER decoding above `der-tlv`.
- Added exact canonical BOOLEAN, INTEGER, BIT STRING, OCTET STRING, NULL, and
  OBJECT IDENTIFIER value handling.
- Added bounded SEQUENCE, SET, and explicit context-wrapper traversal with one
  shared depth and total-element budget.
- Added redacted error categories, transactional cursor accounting, checked
  integer conversion, infallible validated OID iteration, and adversarial tests.
- Added universal IA5String validation and exact schema-selected implicit
  OCTET STRING, IA5String, and OBJECT IDENTIFIER decoding for later X.509
  context-specific values.
- Added package-native execution of all 122 language-neutral DER ASN.1 v1
  cases, including all 46 referenced DER TLV exact-decode cases and redaction
  checks.
