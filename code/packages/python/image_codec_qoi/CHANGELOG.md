# Changelog — coding-adventures-image-codec-qoi

## 0.1.0 — 2026-04-05

Initial release.

- `QoiCodec` — `ImageCodec` implementation for QOI
- `encode_qoi(pixels)` — encode `PixelContainer` to QOI bytes
- `decode_qoi(data)` — decode QOI bytes to `PixelContainer`
- All six ops implemented: OP_RGB, OP_RGBA, OP_INDEX, OP_DIFF, OP_LUMA, OP_RUN
- Hash: `(r*3 + g*5 + b*7 + a*11) % 64`
- Validates magic, dimensions, and payload size before allocating
- 17 tests, 100% coverage
