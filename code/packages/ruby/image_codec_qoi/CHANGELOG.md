# Changelog — ImageCodecQoi (Ruby)

All notable changes are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versioning: [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0] — 2026-04-05

### Added

- **`ImageCodecQoi.encode_qoi(container)`** — Encodes an RGBA8 PixelContainer to a QOI
  binary string. Implements all 6 ops in priority order: RUN, INDEX, DIFF, LUMA, RGB, RGBA.
  Produces a correct 14-byte header and 8-byte end marker.
- **`ImageCodecQoi.decode_qoi(data)`** — Decodes a QOI binary string to an RGBA8
  PixelContainer. Validates magic bytes and handles all 6 chunk types.
- **`ImageCodecQoi.pixel_hash(r, g, b, a)`** — Running-array index:
  `(r*3 + g*5 + b*7 + a*11) % 64`.
- **`ImageCodecQoi.wrap(delta)`** — Signed byte delta helper:
  `((delta & 0xFF) + 128) & 0xFF - 128`.
- **`ImageCodecQoi::QoiCodec`** — Class implementing the `ImageCodec` mixin interface.
  MIME type: `"image/qoi"`.
- `MAGIC = "qoif"`, `END_MARKER = [0,0,0,0,0,0,0,1].pack("C*")` constants.
- 25 minitest tests covering: header fields, end marker, roundtrips (1×1, solid, grid,
  alpha), RUN op compactness, RUN max-62 boundary, INDEX reuse, DIFF small deltas,
  LUMA medium deltas, and decode error cases.

### Dependencies

- `coding-adventures-pixel-container` (loaded via LOAD_PATH)

### Notes

- RUN encodes lengths 1–62 as 6-bit value (bias -1); 62 is the hard maximum per the spec.
- DIFF uses 2-bit fields biased by -2 (range -2..1 per channel).
- LUMA byte 1 biases green by -32; byte 2 biases dr-dg and db-dg by -8.
- Header width/height are big-endian uint32 (pack `"N"`), unlike BMP's little-endian.
