# @coding-adventures/layout-grid

CSS Grid layout algorithm for the layout pipeline — two-dimensional grid
layout for tables, card grids, and magazine layouts.

```
LayoutNode (with ext["grid"]) + Constraints + TextMeasurer
    ↓  layout_grid()
PositionedNode  →  layout-to-paint  →  paint-vm-canvas
```

See: `code/specs/UI08-layout-grid.md`

## Installation

```bash
npm install @coding-adventures/layout-grid
```

## Usage

```ts
import { layout_grid } from "@coding-adventures/layout-grid";
import { constraints_width } from "@coding-adventures/layout-ir";

const table = {
  // ...
  ext: {
    grid: {
      templateColumns: "repeat(3, 1fr)",
      rowGap: 8,
      columnGap: 8,
    },
  },
};
const result = layout_grid(table, constraints_width(900), measurer);
```

## API

### `layout_grid(container, constraints, measurer) → PositionedNode`

| Parameter | Type | Description |
|-----------|------|-------------|
| `container` | `LayoutNode` | Grid container with `ext["grid"]` |
| `constraints` | `Constraints` | Parent size constraints |
| `measurer` | `TextMeasurer` | Text measurement provider |

### Track List Syntax

Track lists are CSS-like strings:

| Syntax | Meaning |
|--------|---------|
| `"200px"` | Fixed 200px track |
| `"1fr"` | Flexible: proportional share of remaining space |
| `"auto"` | Sized to content |
| `"minmax(100px, 1fr)"` | At least 100px, at most 1fr |
| `"repeat(3, 1fr)"` | Three equal flexible tracks |
| `"100px 1fr 100px"` | Three tracks: fixed, flex, fixed |

### `GridContainerExt`

```ts
ext: {
  grid: {
    templateColumns: "repeat(3, 1fr)",  // default: "1fr"
    templateRows: "auto",               // default: "auto"
    columnGap: 16,                      // default: 0
    rowGap: 8,                          // default: 0
    autoRows: "auto",                   // implicit row size
    autoColumns: "auto",                // implicit column size
    autoFlow: "row",                    // "row" | "column" | "dense"
    alignItems: "stretch",              // "start" | "center" | "end" | "stretch"
    justifyItems: "stretch",            // "start" | "center" | "end" | "stretch"
  }
}
```

### `GridItemExt`

```ts
ext: {
  grid: {
    columnStart: 1,       // 1-based column line
    columnEnd: 3,         // exclusive end
    columnSpan: 2,        // alternative to columnEnd
    rowStart: 2,
    rowSpan: 1,
    alignSelf: "center",  // overrides container alignItems
    justifySelf: "end",   // overrides container justifyItems
  }
}
```

## Related Packages

- `@coding-adventures/layout-ir` — shared types
- `@coding-adventures/layout-flexbox` — CSS Flexbox algorithm
- `@coding-adventures/layout-block` — Block/inline flow algorithm
- `@coding-adventures/layout-to-paint` — converts positioned nodes to paint instructions
- `@coding-adventures/document-ast-to-layout` — generates grid layouts from table nodes
