# UI03 — layout-flexbox: Flexbox Layout Algorithm

## Overview

`layout-flexbox` is a pure layout algorithm. It takes a list of `LayoutNode`
children, reads the flex properties from each node's `ext["flex"]` field, and
produces a `PositionedNode[]` with resolved positions and sizes.

```
LayoutNode[] + Constraints + TextMeasurer
    ↓  layout_flexbox()
PositionedNode[]
```

The algorithm follows the CSS Flexible Box Layout specification (Level 1),
covering the subset of features needed for the Mosaic Column/Row/Spacer model
and general-purpose UI layout. It does not implement every edge case of the CSS
spec — only the subset documented here is guaranteed.

---

## Package: `layout-flexbox`

**Depends on:** `layout-ir`

**Exports:** `layout_flexbox`, `FlexExt`, `FlexContainerExt`, `FlexItemExt`

---

## Extension schema

Flex properties live in `ext["flex"]`. The value is a map with two sub-schemas:
container properties (read from the **parent** node being laid out) and item
properties (read from each **child** node).

### `FlexContainerExt` — read from the parent node

```
FlexContainerExt {
  direction:       "row" | "column"           // default: "column"
  wrap:            "nowrap" | "wrap"          // default: "nowrap"
  alignItems:      "start" | "center" | "end" | "stretch"   // default: "stretch"
  justifyContent:  "start" | "center" | "end" | "between" | "around" | "evenly"
                                              // default: "start"
  gap:             float                      // uniform gap between items, default 0
  rowGap:          float                      // gap between rows (wrap), default = gap
  columnGap:       float                      // gap between columns, default = gap
}
```

### `FlexItemExt` — read from each child node

```
FlexItemExt {
  grow:    float    // flex-grow factor, default 0
  shrink:  float    // flex-shrink factor, default 1
  basis:   SizeValue?   // flex-basis; null = use node's width/height, default null
  alignSelf: "start" | "center" | "end" | "stretch" | "auto"
                    // overrides parent alignItems; "auto" = inherit, default "auto"
  order:   int      // paint order within flex line, default 0 (lower = earlier)
}
```

Both schemas live under the same `ext["flex"]` key as a flat map:

```
ext["flex"] = {
  // container fields
  "direction": "row",
  "gap": 8,
  "alignItems": "center",
  // item fields on the same node (it can be both a container and an item)
  "grow": 1
}
```

---

## Function signature

```
layout_flexbox(
  container: LayoutNode,        // the flex container node
  constraints: Constraints,     // available space
  measurer: TextMeasurer        // for measuring text leaf nodes
) → PositionedNode
```

The function takes the **container node**, not just its children. It reads
`ext["flex"]` from the container for flex container properties, recursively
positions children within the container's content area (inner bounds after
padding), and returns a single `PositionedNode` for the container with its
children fully positioned.

---

## Algorithm

The algorithm follows the standard flex layout steps. All measurements are in
logical units throughout.

### Step 1 — Resolve container size

Determine the container's own width and height:

1. If `container.width` is `fixed(v)` → container width = v
2. If `container.width` is `fill` → container width = `constraints.maxWidth`
3. If `container.width` is `wrap` or null → provisionally unconstrained; will
   be resolved after children are measured (wrap to content)
4. Same logic for height
5. Apply `min/maxWidth` and `min/maxHeight` clamps after the above

### Step 2 — Determine main and cross axis

- `direction = "row"` → main axis = horizontal, cross axis = vertical
- `direction = "column"` → main axis = vertical, cross axis = horizontal

### Step 3 — Collect items and determine hypothetical main sizes

For each child in `container.children`, in `order` sort:

1. Read `FlexItemExt` from `child.ext["flex"]` (or use defaults if absent)
2. Determine the child's **hypothetical main size**:
   - If `basis` is set: use `basis` as the starting size along the main axis
   - Otherwise: use the child's `width` (row) or `height` (column) as SizeValue
   - `fixed(v)` → v
   - `fill` → treat as 0 for the hypothetical (growth resolved in step 5)
   - `wrap` → measure the child to find its natural size (call `measure_node`)
3. Apply `min/maxWidth` and `min/maxHeight` clamps

### Step 4 — Collect into flex lines

- `wrap = "nowrap"` → single line containing all children
- `wrap = "wrap"` → greedily fill lines: add children to current line until
  main-axis sum + gap exceeds available main-axis space, then start a new line

### Step 5 — Resolve flexible lengths (grow / shrink)

For each flex line:

1. Compute **free space** = container main size − sum of hypothetical main sizes
   − sum of gaps between items
2. If free space > 0 and any child has `grow > 0`:
   - Distribute free space proportionally to `grow` factors
   - Add distributed amount to each child's main size
3. If free space < 0 and any child has `shrink > 0`:
   - Distribute the deficit proportionally to `shrink × hypothetical_main_size`
     (standard CSS shrink behavior)
   - Subtract distributed amount from each child's main size
4. Clamp each child's final main size against its `min/maxWidth`/`min/maxHeight`

### Step 6 — Resolve cross sizes

For each child on each line, determine its cross-axis size:

1. Check `alignSelf`:
   - `"auto"` → use parent's `alignItems`
   - Otherwise → use child's own `alignSelf`
2. If resolved alignment is `"stretch"`:
   - Set child cross size = line cross size (full stretch)
   - Exception: if child has an explicit cross-axis size hint (fixed), use that
3. Otherwise:
   - Measure the child at its resolved main size to find natural cross size
   - Apply `min/maxHeight`/`min/maxWidth` clamps

### Step 7 — Determine main-axis positions (justifyContent)

Place items along the main axis within each line:

- `"start"` → pack toward the start of the main axis
- `"end"` → pack toward the end
- `"center"` → center the packed group
- `"between"` → equal spacing between items, none at edges
- `"around"` → equal spacing around each item
- `"evenly"` → equal spacing between items and at edges

Apply `gap`/`columnGap`/`rowGap` between items regardless of `justifyContent`.

### Step 8 — Determine cross-axis positions (alignItems / alignSelf)

For each item within its line:

- `"start"` → item cross start = line cross start
- `"end"` → item cross start = line cross end − item cross size
- `"center"` → item cross start = line cross start + (line cross size − item cross size) / 2
- `"stretch"` → already resolved in step 6; item cross start = line cross start

### Step 9 — Determine line positions (multi-line only)

If `wrap = "wrap"`, stack lines along the cross axis. Currently: lines are
packed start-to-start with `rowGap` between them. (Align-content variants
are a future extension.)

### Step 10 — Recursively lay out children

For each child with its now-resolved (x, y, width, height):

1. Build a new `Constraints` from the resolved size:
   `constraints_fixed(resolved_width, resolved_height)`
2. If the child has `children` of its own: the caller must choose how to lay
   out the subtree. `layout_flexbox` only positions its **direct children**.
   It does not recurse automatically — the pipeline is the caller's responsibility.
3. If the child is a leaf (text or image): produce a `PositionedNode` directly
   with its resolved geometry and `content` carried through unchanged.

### Step 11 — Apply padding to container

Shift all child positions by the container's `padding.left` and `padding.top`.
Add `padding.right`/`padding.bottom` to the container's content area when
computing the `fill` size.

### Step 12 — Return container PositionedNode

```
PositionedNode {
  x: 0, y: 0,           // caller positions the container itself
  width: resolved_width,
  height: resolved_height,
  id: container.id,
  content: null,
  children: [positioned child nodes],
  ext: container.ext
}
```

---

## `measure_node` helper

The algorithm internally calls `measure_node` to find the natural size of a
child when its size is `wrap` or the flex-basis is unspecified.

```
measure_node(node: LayoutNode, constraints: Constraints, measurer: TextMeasurer) → Size

Size { width: float, height: float }
```

For leaf nodes:
- `TextContent` → call `measurer.measure(value, font, maxWidth)` where
  `maxWidth = constraints.maxWidth` (or null if unconstrained). Return
  `{ width: result.width, height: result.height }`.
- `ImageContent` → return `{ width: constraints.maxWidth, height: constraints.maxWidth }`
  (square by default; a fixed size hint on the node overrides this).

For container nodes:
- Recursively measure the subtree under its natural size. The exact algorithm
  is up to the caller — `measure_node` for a container node calls `layout_flexbox`
  with unconstrained width or height and returns the resulting size.

---

## What this package does NOT do

- Does not choose the layout algorithm for child containers
- Does not validate `ext["flex"]` fields — missing fields use defaults, unknown
  fields are ignored
- Does not implement CSS `position: absolute` or `position: fixed`
- Does not implement `flex-flow` shorthand parsing (use `direction` + `wrap`)
- Does not implement `align-content` for multi-line containers (future extension)
- Does not implement `z-index` (handled by `paint-ir` layer ordering)
- Does not manage fonts or load resources
