# Changelog — @coding-adventures/image-codec-ppm

## 0.1.0 — 2026-04-05

Initial release.

- `PpmCodec` class implementing `ImageCodec`
- `encodePpm(pixels)` — PPM P6 bytes; alpha dropped (PPM has no alpha channel)
- `decodePpm(bytes)` — decoded pixels have A=255; handles '#' comment lines
- 13 tests, all passing
