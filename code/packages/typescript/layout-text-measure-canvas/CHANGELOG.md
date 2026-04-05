# Changelog — @coding-adventures/layout-text-measure-canvas

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
