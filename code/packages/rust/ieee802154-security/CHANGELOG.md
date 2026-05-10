# Changelog

## 0.1.0

- Initial IEEE 802.15.4 nonce construction primitives.
- Added replay window state for monotonic incoming frame counters.
- Added key identifier normalization and in-memory key lookup scaffolding.
- Added secured-frame material extraction for nonce, authenticated header bytes,
  encrypted payload bytes, and MIC bytes.
- Added first AES-CCM* encrypt/decrypt implementation with RFC 3610 vector
  coverage and constant-time MIC comparison.
- Added outgoing frame counter allocation per source/key tuple with restored
  next-counter support for future persistent stores.
