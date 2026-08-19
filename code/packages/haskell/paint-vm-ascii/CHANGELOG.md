# Changelog

## 0.2.0

- Implement the full `P2D02-paint-vm-ascii.md` contract: add `line`,
  `glyph_run`, `group`, `clip`, and `layer` instruction handling (previously
  `rect`-only).
- Rect rendering now supports `stroke` (box-drawing corners/edges) in
  addition to `fill`; both use a shared cell-flag tag system so intersecting
  strokes from a rect and a line merge into the correct box-drawing glyph
  regardless of draw order.
- `group`/`layer` reject non-identity transforms, non-default opacity,
  filters, and non-normal blend modes via `Left (UnsupportedInstruction ...)`,
  matching every other language port's "fail loudly" behavior.
- Rewrote the render buffer from a `[String]` list to a sparse
  `Map (Int, Int) Cell`, since box-drawing merge logic needs per-cell tag
  state that a plain character grid can't represent.
- This backend is now consumed by the Haskell `cowsay` port (see
  `code/specs/cowsay-paintvm-pipeline.md`).
- Hardening (found across three rounds of `/security-review`):
  - `line` geometry is now validated the same way `rect` geometry already
    was (`Left (InvalidLineGeometry ...)` for NaN/Infinite coordinates); rect
    fill/stroke ranges and line endpoints are clamped to the active clip's
    bounds *before* any range is built, so a caller-supplied instruction
    with a huge (but finite) extent can't force unbounded iteration or an
    unbounded Bresenham recursion.
  - `glyph_run` rejects UTF-16 surrogate code points (`0xD800`-`0xDFFF`), not
    just C0/C1 controls and bidi-control code points; a glyph with a
    non-finite position is skipped rather than crashing the whole render.
  - `clip` geometry is validated the same way (`Left (InvalidClipGeometry
    ...)`), including the `x+w`/`y+h` extents (two individually-finite
    values near `DBL_MAX` can sum to `+Infinity`) — `rect` gained the same
    extent check.
  - Scene dimensions are now capped (`Left (SceneTooLarge ...)`) both by
    total cell count and *per axis*, since a product-only cap is bypassed by
    a zero-width, huge-height (or vice versa) scene.

## 0.1.0

- Add configurable terminal-cell scaling with shared 8-by-16 defaults.
- Render visible filled rectangles with clipping and whitespace trimming.
- Reject unsupported path instructions explicitly.
- Validate scales, scene dimensions, and rectangle geometry without throwing exceptions.
