# Changelog — @coding-adventures/image-codec-qoi

## 0.1.0 — 2026-04-05

Initial release.

- `QoiCodec` class implementing `ImageCodec`
- `encodeQoi(pixels)` — all 6 QOI operations: OP_RGB, OP_RGBA, OP_INDEX, OP_DIFF, OP_LUMA, OP_RUN
- `decodeQoi(bytes)` — pre-flight payload reachability check before allocating
- Hash function: `(r*3 + g*5 + b*7 + a*11) % 64`
- 14 tests, all passing
