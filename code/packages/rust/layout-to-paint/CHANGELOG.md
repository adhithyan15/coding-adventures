# Changelog

## [0.1.0] — initial release

### Added
- `layout_to_paint(&PositionedNode, &LayoutToPaintOptions) -> PaintScene` — the UI04 entry point. Walks the tree in pre-order, accumulating absolute positions, and emits a flat `PaintScene`.
- `LayoutToPaintOptions<S, M, R>` — generic over a matching TXT00 triple (TextShaper, FontMetrics, FontResolver all sharing the same `Handle` associated type). The Rust type system enforces the font-binding invariant at compile time.
- Background + border + corner-radius emission from `ext["paint"]` map — single `PaintRect` carrying fill, stroke, stroke_width, corner_radius.
- Text content: font resolved once per distinct `(family, weight, italic)` via an internal cache. Each TextContent is wrapped at hard `\n` newlines first, then greedy-word-wrapped within `node.width`. Each resulting line is shaped via the caller's TextShaper and emitted as one `PaintGlyphRun` with per-glyph absolute positions (pen-relative shaper output baked into scene coordinates).
- Image content: emits `PaintImage` with `src` unchanged, sizes from the positioned node.
- Metrics used: `units_per_em`, `ascent`, `descent`, `line_gap` → line-height and baseline offset computation, with FontSpec.line_height multiplier applied.
- Color → CSS: layout-ir `Color { r, g, b, a }` (u8) → `"rgb(R, G, B)"` if alpha==255, else `"rgba(R, G, B, A/255)"`.
- Device-pixel-ratio scaling: all positions, widths, heights, font sizes are multiplied by `device_pixel_ratio` before emission.

### v1 simplifications (documented in source comments)
- No per-node padding preserved from the LayoutNode (PositionedNode does not carry it in v1). Text renders at `(node.x, node.y + ascent)` without extra inset — code blocks look flush against their backgrounds.
- No clip push for rounded corners — the rounded background renders correctly; content is not clipped.
- No shadows, opacity, or layer filters.
- No intrinsic image sizing.

### Tests — 12 pass
- Empty container → empty scene.
- `backgroundColor` in ext → PaintRect with correct fill.
- Single text leaf → PaintGlyphRun with 5 glyphs at baseline_x = node.x, baseline_y = node.y + ascent, advances matching the shaper's x_advance.
- Hard newline → two glyph runs, second baseline strictly below first.
- Word-wrap within box width → three lines from 6 words at 40px wide.
- Absolute positioning accumulates through nested containers.
- Device pixel ratio 2.0 scales font size and positions.
- Failing resolver silently drops text content (scene still valid, no crash).
- Color → CSS round-trip for opaque and alpha channels.
- Image content → PaintImage with src unchanged.
- Font cache: resolve() called once for two siblings with identical FontSpec.

### Dependencies
- `layout-ir`, `paint-instructions`, `text-interfaces` — pure types, no external deps.
