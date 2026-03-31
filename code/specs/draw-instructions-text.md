# Draw Instructions Text

## Overview

This spec defines an ASCII/Unicode text renderer for the draw-instructions
scene model. It is a `DrawRenderer<string>` that converts a 2D scene into
box-drawing characters, proving that the draw-instructions abstraction
works across both visual (SVG, Canvas, Metal) and text-based backends.

The renderer maps pixel-coordinate scenes to a fixed-width character grid.
Rectangles become box-drawing outlines or filled blocks. Lines become
horizontal or vertical strokes. Text is written directly as characters.
The output is a plain string that can be printed to any terminal.

## Why This Matters

If the same `DrawScene` that produces an SVG document or paints a Canvas
can also render as ASCII art, the abstraction is truly backend-neutral.
This is the litmus test for the draw-instructions architecture: any
future backend (Direct2D, Metal, Vulkan, PDF) only needs to implement
the same five primitive handlers.

## Scale Factor

Scene coordinates are in pixels (e.g., a rect at x=0, width=200). The
text renderer maps pixels to characters using a scale factor:

```
charCol = Math.round(sceneX / scaleX)
charRow = Math.round(sceneY / scaleY)
```

Defaults (matching typical monospace font metrics):
- scaleX = 8 pixels per character width
- scaleY = 16 pixels per character height

A 200px-wide rect becomes 25 characters. A 32px-tall row becomes 2 lines.

The scale factor is configurable so producers can tune the resolution.

## Character Mapping

### Filled rectangles (fill set, no stroke)

```
████████████
████████████
████████████
```

Uses the Unicode full block character (U+2588). The entire interior is
filled.

### Stroked rectangles (stroke set)

```
┌──────────┐
│          │
│          │
└──────────┘
```

Uses box-drawing characters:
- `┌` top-left corner (U+250C)
- `┐` top-right corner (U+2510)
- `└` bottom-left corner (U+2514)
- `┘` bottom-right corner (U+2518)
- `─` horizontal line (U+2500)
- `│` vertical line (U+2502)

The interior is left as spaces.

### Filled + stroked rectangles (both fill and stroke)

Same as stroked — the border characters take priority over fill.

### Lines

Horizontal lines use `─` (U+2500). Vertical lines use `│` (U+2502).
Diagonal lines are approximated with the nearest character.

### Intersections

When a horizontal and vertical line cross, the renderer uses `┼` (U+253C).
When a line meets a box edge:
- Top edge: `┬` (U+252C)
- Bottom edge: `┴` (U+2534)
- Left edge: `├` (U+251C)
- Right edge: `┤` (U+2524)

This produces clean table grids:

```
┌──────────┬─────┬────────┐
│ Name     │ Age │ City   │
├──────────┼─────┼────────┤
│ Alice    │  30 │ NYC    │
│ Bob      │  25 │ LA     │
└──────────┴─────┴────────┘
```

### Text

Text characters are written directly into the buffer at their mapped
position. Text alignment:
- `start`: text starts at the mapped x position
- `middle`: text is centered on the mapped x position
- `end`: text ends at the mapped x position

Bold text (fontWeight: "bold") is rendered identically to normal text
in the character grid — there is no bold in fixed-width ASCII. The
font weight is silently ignored.

### Clips

Clip instructions constrain drawing to a rectangular region. Characters
that would fall outside the clip bounds are not written to the buffer.

### Groups

Groups simply recurse into their children — no visual effect.

## Rendering Algorithm

```
1. Compute buffer dimensions:
   cols = ceil(scene.width / scaleX)
   rows = ceil(scene.height / scaleY)

2. Create buffer: rows × cols, filled with spaces

3. Walk instructions in order:
   For each instruction:
     rect  → write box-drawing chars or fill chars to buffer
     line  → write ─ or │ chars along the line path
     text  → write text string at mapped position
     clip  → set clip bounds, recurse into children, restore bounds
     group → recurse into children

4. Handle intersections:
   When writing a box-drawing char, check what's already in the cell.
   If crossing characters exist, upgrade to the appropriate junction
   (e.g., ─ crossing │ becomes ┼)

5. Join rows with newlines, trim trailing spaces, return string
```

## Public API

```typescript
interface TextRendererOptions {
  scaleX?: number;  // pixels per character width (default: 8)
  scaleY?: number;  // pixels per character height (default: 16)
}

function createTextRenderer(options?: TextRendererOptions): DrawRenderer<string>;
function renderText(scene: DrawScene, options?: TextRendererOptions): string;

const TEXT_RENDERER: DrawRenderer<string>;  // default scale
```

## Implementation Notes

- The buffer is a 2D array of single characters, not a string.
  Writing to specific positions is O(1).
- Intersection detection uses a parallel "tag" buffer that tracks
  what kind of element is at each cell (horizontal line, vertical line,
  corner, etc.). This enables correct junction character selection.
- Trailing whitespace on each line is trimmed before joining.
- The background color of the scene is ignored — the terminal provides
  its own background.
- Fill colors are ignored — all filled rects use the same block char.
  (A future extension could use ANSI color codes.)
