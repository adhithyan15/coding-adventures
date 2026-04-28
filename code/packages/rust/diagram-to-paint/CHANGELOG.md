# Changelog — diagram-to-paint

## 0.1.2 — Fix text coordinate-space mismatch (text now inside nodes)

### Fixed
- **Text rendered below/off canvas on Retina** — `layout_to_paint` with `device_pixel_ratio > 1`
  emits glyph positions in device pixels, but `paint-metal` creates its CGBitmap at `scene.height`
  logical pixels and flips y as `height − gy`. With DPR=2 the glyph y (≈106 dp) exceeded the
  100-logical-pixel canvas height, placing text off the bottom edge. Fixed by passing
  `device_pixel_ratio: 1.0` to the text bridge so glyph coordinates stay in the same logical-pixel
  space as all node/edge geometry. A future DPR-aware pass can scale the full scene consistently.

## 0.1.1 — Real text shaping via layout-to-paint

### Changed (breaking)
- `DiagramToPaintOptions` is now a **generic struct** with lifetime:
  `DiagramToPaintOptions<'a, S: TextShaper, M: FontMetrics, R: FontResolver>`.
  The `shaper`, `metrics`, and `resolver` fields replace the old `ps_font_name` field.
  `background` is now a `layout-ir::Color` (RGBA) instead of a CSS string.
  New fields: `device_pixel_ratio`, `label_font: FontSpec`, `title_font: FontSpec`.
- `diagram_to_paint` is now generic over the TXT00 triple:
  `fn diagram_to_paint<S, M, R>(diagram, options: &DiagramToPaintOptions<'_, S, M, R>) -> PaintScene`.
- All text (node labels, edge labels, diagram title) is now rendered via
  `layout-to-paint::layout_to_paint`. A `PositionedNode` tree is built for all text items
  (one node per label/title) and passed to `layout_to_paint` in a single call. This produces
  `PaintGlyphRun` instructions with **real glyph IDs** from the font shaper, not Unicode codepoints.
  `TextAlign::Center` is used for all text nodes.
- Painter's algorithm order: edges (lines + arrowheads) → node shapes → text labels.
  Node shapes are still emitted directly as `PaintRect`/`PaintEllipse`/`PaintPath`.
- Added dependencies: `layout-ir`, `layout-to-paint`, `text-interfaces`.
- Removed: `coretext_font_ref`, `approx_char_advance`, `centred_glyph_run` helpers —
  text rendering is fully delegated to `layout-to-paint`.

### Tests — 15 pass (was 11)
- Tests now use a `FakeShaper`/`FakeMetrics`/`FakeResolver` triple (same pattern as
  `layout-to-paint`'s tests). The `make_opts` helper constructs a `DiagramToPaintOptions`.
- `glyph_run_font_ref_is_shaper_provided` — asserts `font_ref == "fake:test"`, verifying
  glyph IDs come from the shaper rather than a hardcoded `coretext:` string.
- `painter_order_edges_before_nodes` — asserts all `PaintPath` (edges) indices < first
  `PaintRect` (node shape) index, enforcing the z-order invariant.
- `css_to_color_parses_hex` — covers the new `css_to_color` helper.
- `edge_label_produces_glyph_run` — compares run count with/without an edge label.

## 0.1.0

Initial release.

- `diagram_to_paint(diagram, options) -> PaintScene` — main entry point
- `DiagramToPaintOptions` — background colour, CoreText PS font name, title font size
- Node shape rendering: Rect → PaintRect, RoundedRect → PaintRect with corner_radius,
  Ellipse → PaintEllipse, Diamond → PaintPath (4-vertex polygon)
- Node labels via PaintGlyphRun with `coretext:` font scheme
- Edge polylines via PaintPath (stroke only, round caps and joins)
- Directed edge arrowheads via filled PaintPath triangle
- Edge labels via PaintGlyphRun
- Diagram title via PaintGlyphRun centred at top of canvas
- Edges rendered before nodes (correct z-order: edges behind nodes)
