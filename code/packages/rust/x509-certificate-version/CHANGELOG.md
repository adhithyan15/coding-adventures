# Changelog

## 0.1.0 — 2026-09-20

- Added allocation-free decoding for the explicit RFC 5280 certificate version
  wrapper.
- Added a caller-visible omitted-field `V1` default while rejecting an
  explicitly encoded DER default and unsupported integer values.
- Added adversarial wrapper, integer, shared-budget, and redacted-error tests.
