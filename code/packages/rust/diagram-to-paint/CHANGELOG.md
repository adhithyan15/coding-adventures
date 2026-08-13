# Changelog — diagram-to-paint

## Unreleased

- Validate composed state classes through resolved graph styles and native Metal PNG rendering.
- Validate multiline labels from quoted state aliases with trailing descriptions through native Metal PNG rendering.
- Validate state hash comments alongside semantic colors through native Metal PNG rendering.
- Omit empty graph-state shapes and labels when the semantic directive requests it.
- Shape multiline graph-node descriptions without backend soft rewrapping.
- Lower concurrent state-region dividers to backend-neutral Paint paths.
- Lower resolved composite graph-group colors and stroke geometry to Paint.
- Lower composite graph groups to backend-neutral background rectangles and shaped labels.
- Export graph node URLs, tooltips, and hit-test bounds through PaintScene metadata.
- Export graph-family accessibility metadata through PaintScene metadata.
- Lower graph note nodes and dashed note associations to backend-neutral paths.
- Lower compact graph-IR bar nodes to backend-neutral rectangles for state fork/join rendering.
- Render destroyed participants through their message-positioned footer geometry instead of adding an unconditional destruction cross.
- Resolve self-message source and destination tips independently for reverse/bidirectional arrowheads and central endpoint markers.
- Paint sequence activation bars behind message paths so arrowheads remain visible at activation edges.
- Validate message-bound sequence create/destroy events through Metal PNG rendering.
- Render depth-offset nested sequence activation bars through backend-neutral rectangles.
- Validate resumed, two-decimal sequence autonumber counters through Metal PNG rendering.
- Validate mixed-case sequence syntax through Metal PNG rendering.
- Validate escaped participant configuration aliases through Metal PNG rendering.
- Validate comma-bearing participant configuration aliases through Metal PNG rendering.
- Render mirrored sequence footer participants with backend-neutral instructions.
- Validate sequence hash comments and adjacent entities through native Metal PNG rendering.
- Render grammar-backed sequence `actor` declarations as backend-neutral UML stick figures.
- Validate ordered sequence autonumber toggles and resets through PaintScene and Metal PNG.
- Shape resolved multiline sequence participant-box labels without backend soft rewrapping.
- Shape resolved multiline sequence participant aliases without backend soft rewrapping.
- Shape resolved multiline sequence control labels without backend soft rewrapping.
- Validate hyphenated sequence actor IDs through the native Metal PNG pipeline.
- Validate multiword sequence actor IDs through the native Metal PNG pipeline.
- Validate forced sequence text wrapping through PaintScene and Metal PNG rendering.
- Export sequence accessibility title and description as PaintScene metadata.
- Preserve multiline accessibility descriptions in that scene metadata.
- Validate semicolon-separated sequence input through Metal PNG rendering.
- Validate normalized HSL sequence group colors through Metal PNG rendering.
- Validate legacy colon-prefixed sequence titles through Metal PNG rendering.
- Validate decoded sequence entity characters through native text shaping.
- Shape multiline sequence messages and notes into reserved layout geometry.
- Export sequence actor details references as PaintScene metadata.
- Export JSON-valued sequence actor properties as PaintScene metadata.
- Export sequence actor links as PaintScene hit-test metadata.
- Paint sequence rect blocks with their declared functional colors.
- Format decimal sequence numbers without redundant trailing zeroes.
- Added central-connection endpoint circles layered above activation bars.
- Added normal and reverse filled/stick sequence half-arrow geometry.
- Added backend-neutral sequence symbols for boundary, control, entity,
  database, collections, and queue participants.
- Added sequence participant-group backgrounds and labels.
- Added a Mermaid Pie -> chart layout -> PaintScene -> Metal PNG example and
  Apple end-to-end test.
- Added sequence lowering for participant headers, lifelines, messages,
  arrowheads, notes, activation bars, and shaped labels, plus a Metal PNG test.
- Added nested sequence frame and branch-divider lowering with visual Metal coverage.
- Added sequence destruction markers and dynamic participant Metal coverage.

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
