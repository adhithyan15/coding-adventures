# Changelog — ImageCodecPpm (Ruby)

All notable changes are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versioning: [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0] — 2026-04-05

### Added

- **`ImageCodecPpm.encode_ppm(container)`** — Encodes an RGBA8 PixelContainer to a
  binary P6 PPM file. Produces a plain-text header (`P6\n<w> <h>\n255\n`) followed
  by raw RGB bytes (3 bytes per pixel). Alpha channel is dropped.
- **`ImageCodecPpm.decode_ppm(data)`** — Parses a P6 PPM binary string into an RGBA8
  PixelContainer. Skips `#` comment lines in the header; validates magic and maxval.
  All decoded pixels receive A = 255.
- **`ImageCodecPpm::PpmCodec`** — Class implementing the `ImageCodec` mixin interface.
  MIME type: `"image/x-portable-pixmap"`.
- 22 minitest tests covering header format, pixel order, encode/decode roundtrips,
  comment-line handling, dimension preservation, and error cases.

### Dependencies

- `coding-adventures-pixel-container` (loaded via LOAD_PATH)
