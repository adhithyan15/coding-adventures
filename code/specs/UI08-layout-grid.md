# UI08 — layout-grid: CSS Grid Layout Algorithm

## Overview

`layout-grid` implements the CSS Grid Layout specification — the subset needed
for table-like structures, magazine layouts, and card grids. It reads
`GridExt` from each node's `ext["grid"]` and produces a `PositionedNode` tree
with all grid items placed in their resolved cells.

```
LayoutNode[] + Constraints + TextMeasurer
    ↓  layout_grid()
PositionedNode[]
```

The primary initial consumer is `document-ast-to-layout` (for table rendering).
Future consumers include Mosaic grid components and any producer that generates
grid-based layouts.

---

## Package: `layout-grid`

**Depends on:** `layout-ir`

**Exports:** `layout_grid`, `GridExt`, `GridContainerExt`, `GridItemExt`

---

## Extension schema

Grid properties live in `ext["grid"]`.

### `GridContainerExt` — read from the container node

```
GridContainerExt {
  templateColumns:  TrackList    // column track sizes, default "1fr"
  templateRows:     TrackList    // row track sizes, default "auto"
  columnGap:        float        // gap between columns, default 0
  rowGap:           float        // gap between rows, default 0
  autoRows:         TrackSize    // size for implicitly created rows, default "auto"
  autoColumns:      TrackSize    // size for implicitly created columns, default "auto"
  autoFlow:         "row" | "column" | "dense"   // default "row"
  alignItems:       "start" | "center" | "end" | "stretch"   // default "stretch"
  justifyItems:     "start" | "center" | "end" | "stretch"   // default "stretch"
}
```

### `GridItemExt` — read from each child node

```
GridItemExt {
  columnStart:  int | "auto"    // 1-based column line number, default "auto"
  columnEnd:    int | "auto"    // exclusive end column line, default "auto"
  columnSpan:   int             // number of columns to span, default 1
  rowStart:     int | "auto"    // 1-based row line number, default "auto"
  rowEnd:       int | "auto"    // exclusive end row line, default "auto"
  rowSpan:      int             // number of rows to span, default 1
  alignSelf:    "start" | "center" | "end" | "stretch" | "auto"  // default "auto"
  justifySelf:  "start" | "center" | "end" | "stretch" | "auto"  // default "auto"
}
```

### `TrackList`

A track list describes column or row sizes. It is a string in a simplified
CSS grid track list syntax:

```
TrackList = string
```

Examples:
- `"200px 1fr 100px"` — three tracks: fixed, flexible, fixed
- `"repeat(3, 1fr)"` — three equal flexible tracks
- `"repeat(4, 200px)"` — four fixed tracks
- `"auto"` — single track, sized to content
- `"1fr"` — single flexible track filling all space

### `TrackSize`

A single track size value:

```
TrackSize = string
```

Examples: `"auto"`, `"1fr"`, `"200px"`, `"minmax(100px, 1fr)"`

---

## Function signature

```
layout_grid(
  container: LayoutNode,
  constraints: Constraints,
  measurer: TextMeasurer
) → PositionedNode
```

---

## Algorithm

### Step 1 — Parse track list

Parse `templateColumns` and `templateRows` into sequences of `TrackDefinition`:

```
TrackDefinition =
  | { kind: "fixed",   size: float }         // "200px"
  | { kind: "flexible", fraction: float }    // "1fr", "2fr"
  | { kind: "auto" }                         // size to content
  | { kind: "minmax", min: TrackSize, max: TrackSize }
```

`repeat(N, size)` expands to N copies of the track definition.

### Step 2 — Place items into the grid

For each child, read `GridItemExt`:

1. If `columnStart` and `rowStart` are explicit (not `"auto"`): place directly
   at the specified cell. Span is `columnEnd - columnStart` or `columnSpan`.
2. If either is `"auto"`: use the auto-placement algorithm —
   - `autoFlow: "row"` → advance row-by-row, placing items in the first
     available cell that fits the item's span
   - `autoFlow: "column"` → advance column-by-column
   - `autoFlow: "dense"` → backtrack to fill gaps

After placement, the **grid area** for each item is `(rowStart, columnStart,
rowEnd, columnEnd)` — all 1-based line numbers.

### Step 3 — Create implicit tracks

If any item's placement references a row or column beyond the explicit track
count, create implicit tracks using `autoRows` / `autoColumns` definitions.

### Step 4 — Resolve track sizes (intrinsic sizing)

For each track in order:

1. **Fixed tracks** (`"200px"`) → size = fixed value directly
2. **Auto tracks** → find all items whose span covers only this track;
   set track size = max natural size of those items (measure each with
   `measure_node`)
3. **Flexible tracks** (`"1fr"`) → defer until step 5

Apply `minmax()` constraints after initial sizing.

### Step 5 — Distribute free space to flexible tracks

```
total_fixed = sum of all fixed and auto track sizes + all gaps
free_space  = container_width (or height) − total_fixed
total_fractions = sum of all fr values
track_size_per_fraction = free_space / total_fractions
```

Each `fr` track gets `fraction × track_size_per_fraction` logical units.
Minimum: each `fr` track gets at least its minimum content size.

### Step 6 — Compute item positions and sizes

For each item with grid area `(rowStart, columnStart, rowEnd, columnEnd)`:

```
x = sum of column track sizes [0..columnStart-1] + (columnStart-1) × columnGap
y = sum of row track sizes    [0..rowStart-1]    + (rowStart-1)    × rowGap
w = sum of column track sizes [columnStart..columnEnd-1] + spans × columnGap
h = sum of row track sizes    [rowStart..rowEnd-1]       + spans × rowGap
```

Then apply alignment within the cell:

- `justifySelf` (horizontal): `"stretch"` → use full cell width; others → shrink to content and align
- `alignSelf` (vertical): `"stretch"` → use full cell height; others → shrink to content and align

### Step 7 — Return container PositionedNode

```
PositionedNode {
  x: 0, y: 0,
  width:  total column sizes + gaps + container padding.left + padding.right
  height: total row sizes + gaps + container padding.top + padding.bottom
  children: [positioned grid items]
  ext: container.ext
}
```

---

## What this package does NOT do

- Does not implement `grid-template-areas` (named areas)
- Does not implement `grid-auto-flow: dense` with complex gap-filling
  (basic dense is supported, full CSS dense algorithm is future)
- Does not implement `align-content` / `justify-content` (container alignment
  of the grid within its containing block)
- Does not implement subgrid
- Does not validate that items do not overlap (overlap is the caller's
  responsibility)
