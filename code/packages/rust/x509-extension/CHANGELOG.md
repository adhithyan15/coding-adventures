# Changelog

## Unreleased

- Retained compact opaque authority from the checked extension-value OCTET
  STRING so semantic decoders preserve the enclosing DER depth budget without
  inflating every bounded inline extension slot.

## 0.1.0 — 2026-09-20

- Added allocation-free generic RFC 5280 `Extension` decoding.
- Added validated borrowed extension OIDs, canonical omitted-default critical
  flags, and opaque OCTET STRING contents.
- Added redacted local-offset errors and adversarial structure, OID, BOOLEAN,
  framing, OCTET STRING, and shared-budget tests.
