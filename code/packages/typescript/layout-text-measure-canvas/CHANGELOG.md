# Changelog — @coding-adventures/layout-text-measure-canvas

## Unreleased

### Fixed

- Prefer `fontBoundingBoxAscent + fontBoundingBoxDescent` (constant per font+size) for the reported height instead of `actualBoundingBoxAscent + actualBoundingBoxDescent` (which varies with which glyphs appear in the word). The per-glyph envelope caused words like "Heading" (has an ascender 'd') and "on" (x-height only) to report different heights at the same font size, which the inline formatting context then aligned bottoms of — leaving jittery baselines across a line. Falls back to the actualBoundingBox on older engines that do not populate the fontBoundingBox fields.

## 0.1.0 — 2026-04-05

Initial release.

### Added

- `createCanvasMeasurer(ctx)` — factory accepting a `CanvasRenderingContext2D`-compatible context
- `fontSpecToCss(spec)` — converts a `FontSpec` to a CSS font string
- `CanvasContext2D` structural interface for the 2D context (testable without real browser)
- `TextMetricsLike` structural interface for `TextMetrics`
- Single-line measurement using `ctx.measureText()`
- Multi-line word-wrap using greedy left-to-right word splitting
- Fallback height to `font.size × lineHeight` when bounding box metrics are NaN
