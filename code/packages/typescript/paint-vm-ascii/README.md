# @coding-adventures/paint-vm-ascii

Terminal/ASCII backend for `paint-vm`.

This package executes a `PaintScene` into a Unicode character grid using:

- box-drawing characters for strokes and lines
- block characters for fills
- direct character placement for `glyph_run`

## Usage

```typescript
import { paintRect, paintScene } from "@coding-adventures/paint-instructions";
import { renderToAscii } from "@coding-adventures/paint-vm-ascii";

const scene = paintScene(5, 3, "#ffffff", [
  paintRect(0, 0, 4, 2, { fill: "transparent", stroke: "#000000" }),
]);

console.log(renderToAscii(scene, { scaleX: 1, scaleY: 1 }));
```

## Supported kinds

- `rect`
- `line`
- `glyph_run`
- `group`
- `clip`
- plain `layer` values with no filters, transforms, or non-default opacity

Unsupported features fail loudly.
