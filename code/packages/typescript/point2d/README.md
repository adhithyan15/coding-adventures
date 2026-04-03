# @coding-adventures/point2d

2D point/vector and axis-aligned bounding box — the leaf geometric primitive.

## Usage

```typescript
import { Point, Rect } from "@coding-adventures/point2d";

const a = new Point(3, 4);
console.log(a.magnitude());         // ~5
console.log(a.normalize());         // Point { x: 0.6, y: 0.8 }

const r = new Rect(0, 0, 10, 10);
console.log(r.containsPoint(new Point(5, 5))); // true
```

## Layer

G2D00 — depends on `trig` (PHY00).
