# @coding-adventures/draw-instructions-text

ASCII/Unicode text renderer for the draw-instructions scene model. Converts `DrawScene` objects into box-drawing character strings that can be printed to any terminal.

## Usage

```typescript
import { renderText } from "@coding-adventures/draw-instructions-text";
import { createScene, drawRect, drawLine, drawText } from "@coding-adventures/draw-instructions";

const scene = createScene(13, 5, [
  drawRect(0, 0, 12, 4, "transparent", { stroke: "#000", strokeWidth: 1 }),
  drawLine(6, 0, 6, 4, "#000", 1),
  drawLine(0, 1, 12, 1, "#000", 1),
  drawText(1, 0, "Name", { align: "start" }),
  drawText(7, 0, "Age", { align: "start" }),
  drawText(1, 2, "Alice", { align: "start" }),
  drawText(7, 2, "30", { align: "start" }),
]);

console.log(renderText(scene, { scaleX: 1, scaleY: 1 }));
```

Output:
```
┌─────┬─────┐
│Name │Age  │
├─────┼─────┤
│Alice│30   │
└─────┴─────┘
```

## Scale Factor

Scene coordinates are in pixels. The renderer maps pixels to characters:

- `scaleX`: pixels per character column (default: 8)
- `scaleY`: pixels per character row (default: 16)

## Character Palette

| Drawing | Characters |
|---------|-----------|
| Stroked rect | `┌ ┐ └ ┘ ─ │` |
| Filled rect | `█` |
| Horizontal line | `─` |
| Vertical line | `│` |
| Intersection | `┼` |
| Tee junctions | `┬ ┴ ├ ┤` |
