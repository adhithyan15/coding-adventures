# Changelog — image-codec-qoi

## 0.1.0 — 2026-04-05

Initial release.

- `QoiCodec` implementing `ImageCodec` from `pixel-container`
- `encode_qoi(&PixelContainer) → Vec<u8>` convenience function
- `decode_qoi(&[u8]) → Result<PixelContainer, String>` convenience function
- Full QOI spec implementation:
  - `QOI_OP_RGB` (0xFE) — full RGB value, alpha unchanged
  - `QOI_OP_RGBA` (0xFF) — full RGBA value
  - `QOI_OP_INDEX` (tag 00) — hash table back-reference
  - `QOI_OP_DIFF` (tag 01) — 2-bit per-channel delta, bias +2
  - `QOI_OP_LUMA` (tag 10) — 6-bit green delta + 4-bit relative R/B deltas
  - `QOI_OP_RUN` (tag 11) — run-length repeat, max 62 pixels
- Hash function: `(r*3 + g*5 + b*7 + a*11) % 64`
- End marker: `[0,0,0,0, 0,0,0,1]`
- 13 tests, all passing
