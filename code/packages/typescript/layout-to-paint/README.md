# @coding-adventures/layout-to-paint

Converts a `PositionedNode` tree (output of any layout algorithm) into a
`PaintScene` (input to any `paint-vm` backend).

This is the bridge between the **layout** and **paint** layers of the UI
rendering pipeline.

```
layout-flexbox ─┐
layout-block   ─┼──► PositionedNode tree ──► layout-to-paint ──► PaintScene ──► paint-vm-canvas
layout-grid    ─┘
```

See: `code/specs/UI04-layout-to-paint.md`

## Installation

```bash
npm install @coding-adventures/layout-to-paint
```

## Usage

```ts
import { layout_to_paint } from "@coding-adventures/layout-to-paint";
import { rgb } from "@coding-adventures/layout-ir";

const scene = layout_to_paint(positionedNodes, {
  width: 800,
  height: 600,
  background: rgb(255, 255, 255),
  devicePixelRatio: window.devicePixelRatio,
});

// scene.instructions is a PaintInstruction[] ready for paint-vm-canvas
```

## API

### `layout_to_paint(nodes, options) → PaintScene`

| Parameter | Type | Description |
|-----------|------|-------------|
| `nodes` | `PositionedNode[]` | Root-level positioned nodes from a layout pass |
| `options.width` | `number` | Logical canvas width |
| `options.height` | `number` | Logical canvas height |
| `options.background` | `Color \| undefined` | Background fill; defaults to `"transparent"` |
| `options.devicePixelRatio` | `number \| undefined` | HiDPI scale factor; defaults to `1.0` |

Returns a `PaintScene` with `width`, `height`, `background`, and `instructions`.

### `colorToCss(color) → string`

Converts a `Color` value to a CSS `rgba(r,g,b,a)` string. Alpha is normalised
from the 0–255 range to 0–1.

```ts
colorToCss(rgb(255, 0, 0))   // "rgba(255,0,0,1)"
colorToCss(rgba(0, 0, 0, 0)) // "rgba(0,0,0,0)"
```

### `PaintExt`

Attach visual decoration to any `LayoutNode` via `ext["paint"]`:

```ts
const node = container({
  // ...
  ext: {
    paint: {
      backgroundColor: rgb(30, 30, 30),
      borderWidth: 1,
      borderColor: rgb(100, 100, 100),
      cornerRadius: 8,
      opacity: 0.9,
    },
  },
});
```

| Property | Type | Effect |
|----------|------|--------|
| `backgroundColor` | `Color` | Emits a filled `PaintRect` behind children |
| `borderWidth` | `number` | Emits a stroked `PaintRect` (with `borderColor`) |
| `borderColor` | `Color` | Stroke color for the border rect |
| `cornerRadius` | `number` | Rounds corners on background/border rect; clips children |
| `opacity` | `number` | Wraps node instructions in a compositing `PaintLayer` |

## Coordinate System

All positions in `PositionedNode` are **relative to the parent's content area
origin**. `layout-to-paint` recursively accumulates parent offsets so every
paint instruction carries absolute canvas coordinates.

The `devicePixelRatio` is applied at this stage (not in the layout algorithms),
so layout code works entirely in logical pixels.

## Glyph IDs

Text glyphs use Unicode code points as glyph IDs (i.e.,
`char.codePointAt(0)`). This matches the expectation of `paint-vm-canvas`,
which reconstructs characters via `String.fromCharCode(glyph_id)`.

Glyph x positions are estimated using a fixed-width assumption
(`font.size × 0.6` per character). This is intentionally approximate — a
future upgrade can inject a real text measurer.

## Related Packages

- `@coding-adventures/layout-ir` — shared types (`PositionedNode`, `Color`, etc.)
- `@coding-adventures/paint-instructions` — `PaintScene` and instruction types
- `@coding-adventures/paint-vm-canvas` — renders a `PaintScene` onto a `<canvas>`
- `@coding-adventures/layout-flexbox` — CSS Flexbox layout algorithm
- `@coding-adventures/layout-block` — Block/inline flow layout algorithm
