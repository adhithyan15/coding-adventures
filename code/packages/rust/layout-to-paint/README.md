# layout-to-paint

UI04 in Rust. Walks a `PositionedNode` tree and emits a `PaintScene`
with pre-shaped `PaintGlyphRun` instructions.

Spec: [code/specs/UI04-layout-to-paint.md](../../../specs/UI04-layout-to-paint.md)
(the amended version — shaping happens here using the caller-supplied
TXT00 trio; paint backends never re-shape).

## API

```rust
pub fn layout_to_paint<S, M, R>(
    root: &PositionedNode,
    options: &LayoutToPaintOptions<'_, S, M, R>,
) -> PaintScene
where
    S: TextShaper,
    M: FontMetrics<Handle = S::Handle>,
    R: FontResolver<Handle = S::Handle>;
```

The `shaper` + `metrics` + `resolver` triple must share a font binding
— the Rust type system enforces this at compile time via the `Handle`
associated type constraint.

## What it emits

| PositionedNode feature              | PaintInstruction emitted                                       |
|-------------------------------------|---------------------------------------------------------------|
| `ext["paint"]["backgroundColor"]`   | `PaintRect` with fill                                          |
| `ext["paint"]["borderWidth"]` > 0   | `PaintRect` with stroke + stroke_width (same rect as above)    |
| `ext["paint"]["cornerRadius"]`      | `corner_radius` field on the rect                              |
| `Content::Text(tc)`                 | One `PaintGlyphRun` per wrapped line; alignment (Start/Center/End) via `TextContent.text_align` |
| `Content::Image(ic)`                | `PaintImage` with `src` unchanged                              |

All coordinates in the output `PaintScene` are in **device pixels**
(scaled by `device_pixel_ratio`). Colors are CSS `rgb()` / `rgba()`
strings.

## v1 simplifications (documented in source, not hidden)

- **No per-node padding** — layout engine absorbed padding into outer
  dimensions already; text renders at `(node.x, node.y + ascent)`
  without an extra inset. Code blocks look flush against their
  background. Tracked for v2 as a `PositionedNode.padding` field.
- **No clip push** for rounded corners — the rounded background
  renders correctly, content on top is not clipped to the radius.
- **No shadows, opacity, or layer filters** in v1. ext["paint"] fields
  for those are silently ignored.
- **No intrinsic image sizing** — `width`/`height` come directly from
  the node's positioned dimensions.

## Text alignment

`TextContent.text_align` controls how each line is positioned within the node width:

- `TextAlign::Start` — left-aligned (default for document text).
- `TextAlign::Center` — centred; line advance is measured first, then `baseline_x` is computed as `box_x + (width − advance) / 2`. Used by `diagram-to-paint` for all diagram labels.
- `TextAlign::End` — right-aligned.

## Test plan

14 unit tests pass, covering:
- Empty container; background color → Rect; text content →
  PaintGlyphRun with correct ID / baseline / advance math.
- Hard newline → multiple glyph runs with increasing baseline.
- Word-wrap at whitespace within box width → multiple glyph runs.
- Absolute positioning accumulating through two levels of nesting.
- Device pixel ratio scaling of font size + positions.
- Failing resolver drops text silently (no crash).
- Color → CSS conversion for both opaque and alpha channels.
- Image content → PaintImage with src unchanged.
- Font resolution cache: same FontSpec across two nodes → one `resolve()` call.
- Font fallback: shaper producing multiple `ShapedRun`s emits one `PaintGlyphRun`
  per segment with correct font_ref and monotonically increasing x positions.
- Deep tree (1000 levels) does not stack-overflow (iterative walk).
