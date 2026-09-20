# Changelog

## 0.1.0 - 2026-09-20

- Added bounded canonical DER TLV framing and all 54 portable fixtures.
- Validated and snapshotted runtime limit objects so non-finite values and
  caller mutation cannot bypass resource bounds, including accessor-backed
  limits, resizable input buffers, and shadowed typed-array properties.
