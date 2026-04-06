# Changelog — image-codec-qoi (Go)

## 0.1.0 — 2026-04-05

Initial release.

- `QoiCodec` struct implementing `pixelcontainer.ImageCodec`
- `EncodeQoi` — full QOI encoder with all six opcodes
  - OP_INDEX: 64-slot colour hash table
  - OP_DIFF: 2-bit biased deltas per channel
  - OP_LUMA: 6-bit G delta + 4-bit relative R/B deltas
  - OP_RUN: run-length encoding up to 62; automatically flushed at run=62 and end-of-image
  - OP_RGB: full RGB when A unchanged
  - OP_RGBA: full RGBA when A changed
- `DecodeQoi` — full decoder handling all six opcodes
- `IsQoi` — magic-number detection
- `qoiHash` — (r*3 + g*5 + b*7 + a*11) % 64
- `wrap` — signed-delta helper for correct byte wraparound
- 20 unit tests, >95% coverage
