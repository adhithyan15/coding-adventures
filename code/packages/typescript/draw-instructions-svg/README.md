# @ca/draw-instructions-svg

SVG renderer for backend-neutral draw instructions.

This package consumes `@ca/draw-instructions` scenes and returns SVG strings.

## Usage

```typescript
import { createScene, drawRect } from "@ca/draw-instructions";
import { renderSvg } from "@ca/draw-instructions-svg";

const scene = createScene(100, 50, [drawRect(10, 10, 20, 30)]);
const svg = renderSvg(scene);
```
