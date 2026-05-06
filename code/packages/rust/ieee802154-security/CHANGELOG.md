# Changelog

## 0.1.0

- Initial IEEE 802.15.4 nonce construction primitives.
- Added replay window state for monotonic incoming frame counters.
- Added key identifier normalization and in-memory key lookup scaffolding.
- Added secured-frame material extraction for nonce, authenticated header bytes,
  encrypted payload bytes, and MIC bytes.
