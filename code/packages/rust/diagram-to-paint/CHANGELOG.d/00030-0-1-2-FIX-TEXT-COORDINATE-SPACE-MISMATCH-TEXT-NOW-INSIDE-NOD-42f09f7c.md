## 0.1.2 — Fix text coordinate-space mismatch (text now inside nodes)

### Fixed
- **Text rendered below/off canvas on Retina** — `layout_to_paint` with `device_pixel_ratio > 1`
  emits glyph positions in device pixels, but `paint-metal` creates its CGBitmap at `scene.height`
  logical pixels and flips y as `height − gy`. With DPR=2 the glyph y (≈106 dp) exceeded the
  100-logical-pixel canvas height, placing text off the bottom edge. Fixed by passing
  `device_pixel_ratio: 1.0` to the text bridge so glyph coordinates stay in the same logical-pixel
  space as all node/edge geometry. A future DPR-aware pass can scale the full scene consistently.

