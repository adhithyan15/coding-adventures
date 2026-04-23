# P2D02 — PaintVM ASCII Backend

## Overview

`paint-vm-ascii` is the terminal/text backend for the PaintVM family. It
executes a `PaintScene` and produces a plain string made from Unicode
box-drawing characters, block fill characters, and literal text glyphs.

This backend matters for two reasons:

1. It proves the paint IR is not tied to pixel or vector output.
2. It gives the future document pipeline a text-mode target:

```text
Doc
  -> layout tree
  -> PaintScene
  -> paint-vm-ascii
  -> terminal string
```

`paint-vm-ascii` is the canonical text-mode PaintVM backend. The older
`draw-instructions-text` packages are deprecated and should not receive new
features.

---

## Output Model

The backend maps scene coordinates to a fixed-width character grid:

```text
char_col = round(scene_x / scale_x)
char_row = round(scene_y / scale_y)
```

Default scale factors:

- `scale_x = 8`
- `scale_y = 16`

These defaults roughly match monospace glyph metrics and preserve the behavior
of the older text renderer packages.

---

## Character Palette

### Filled rectangles

Filled cells use `█`.

### Stroked rectangles and lines

The backend uses box-drawing characters:

- `┌` `┐` `└` `┘`
- `─` `│`
- `┬` `┴` `├` `┤`
- `┼`

### Glyph runs

`glyph_run` is rendered by converting each `glyph_id` into a Unicode scalar
value and writing that character directly into the grid at the glyph's mapped
position.

This backend intentionally ignores `font_ref`, `font_size`, and `fill`. In a
terminal there is no general way to honor arbitrary font selection or precise
glyph shaping.

This means `paint-vm-ascii` is best suited to glyph runs whose `glyph_id`
values are already ordinary Unicode code points.

---

## Supported Instructions

The canonical backend behavior is:

| Kind | Behavior |
|---|---|
| `rect` | fill and/or stroke via block and box-drawing characters |
| `line` | horizontal/vertical line drawing, diagonal approximation allowed |
| `glyph_run` | direct character placement from `glyph_id` |
| `group` | recurse into children |
| `clip` | intersect clip bounds, recurse into children |
| `layer` | recurse only when it carries no filters, blend mode, transform, or non-default opacity |

Unsupported instructions must fail loudly with an error rather than degrade
silently.

Examples:

- `gradient` -> error
- `image` -> error
- transformed `group` -> error
- filtered `layer` -> error

This matches the PaintVM principle that unsupported instruction kinds or
unsupported backend features are programmer errors, not silent no-ops.

---

## Rendering Rules

### Background

The scene background color is ignored. The terminal provides the background.

### Stroke priority

If a rectangle has both fill and stroke:

- the border cells use box-drawing characters
- interior cells use `█`

### Text priority

Literal glyphs overwrite box-drawing and fill characters already present in a
cell. Later drawing operations must not overwrite an existing glyph cell.

This preserves readable text inside boxes and tables.

### Clipping

`clip` instructions define rectangular clip regions in scene coordinates.
Children can only write within the intersection of the parent clip and the new
clip rectangle.

### Trimming

The final string:

- trims trailing spaces on each line
- trims trailing blank lines at the end of the document

---

## Public API

Every language package should expose:

- an options type with `scale_x` / `scale_y` equivalents
- a convenience `render(scene, options?) -> string`

Languages with a reusable PaintVM host should also expose:

- `createAsciiVM()` or equivalent factory

---

## Porting Guidance

The repository currently has different PaintVM maturity levels across
languages. TypeScript has the full generic `paint-vm` package. Other languages
may expose `paint-vm-ascii` as a direct renderer over `paint-instructions`
until they grow a reusable VM host.

That is acceptable. The important contract is the backend behavior, not that
every port shares the same internal implementation structure on day one.
