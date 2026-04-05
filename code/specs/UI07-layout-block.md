# UI07 — layout-block: Block and Inline Flow Layout Algorithm

## Overview

`layout-block` implements the block and inline flow layout model used by
document renderers. It is the layout algorithm for `document-ast-to-layout`
output, and for any producer that generates block/inline content.

```
LayoutNode[] + Constraints + TextMeasurer
    ↓  layout_block()
PositionedNode[]
```

This is the layout model underlying HTML/CSS normal flow. Block elements stack
vertically. Inline elements flow horizontally and wrap to new lines. The model
is a subset of CSS block formatting context — the subset needed to render
structured documents (Markdown, rich text) correctly.

---

## Package: `layout-block`

**Depends on:** `layout-ir`

**Exports:** `layout_block`, `BlockExt`

---

## Extension schema

Block/inline properties live in `ext["block"]`:

### `BlockExt`

```
BlockExt {
  display:     "block" | "inline"    // default: "block"
  // Block-specific
  float:       "none" | "left" | "right"   // default: "none" (future)
  clear:       "none" | "left" | "right" | "both"  // default: "none" (future)
  // Inline-specific
  verticalAlign: "baseline" | "top" | "middle" | "bottom"  // default: "baseline"
  // Overflow (hint to layout + renderer)
  overflow:    "visible" | "hidden" | "scroll"   // default: "visible"
  whiteSpace:  "normal" | "pre" | "nowrap"       // default: "normal"
  wordBreak:   "normal" | "break-all"            // default: "normal"
}
```

---

## Function signature

```
layout_block(
  container: LayoutNode,
  constraints: Constraints,
  measurer: TextMeasurer
) → PositionedNode
```

Takes the container node (a block-level element), lays out its children
according to block/inline flow rules, and returns a `PositionedNode` with all
children positioned.

---

## Algorithm

### Block formatting context

All direct children of a block container are classified as either block-level
or inline-level based on `ext["block"]["display"]`:

- `"block"` → block-level: stacks vertically, full available width
- `"inline"` → inline-level: flows horizontally, wraps

When a block container has a **mix** of block and inline children, inline
children are wrapped in an anonymous block container (inline formatting
context). This follows the CSS anonymous block generation rule.

### Block layout

For each block-level child in source order:

1. Resolve child width:
   - `size_fill()` → full available width minus child's horizontal margin
   - `size_wrap()` → measure content width, capped at available width
   - `size_fixed(v)` → v, capped by min/maxWidth
2. Compute the child's height by recursively laying it out with
   `constraints_width(resolved_width)`
3. Position child at `(margin.left, current_y + margin.top)`
4. Advance `current_y` by `child.height + margin.top + margin.bottom`
5. Apply `paragraphSpacing` from `ext["block"]` if present, or the node's
   own `margin.bottom`

### Inline formatting context

When laying out a sequence of inline children:

1. Initialize a **line box**: `current_x = 0`, `current_y = 0`,
   `line_height = 0`
2. For each inline child:
   a. Measure the child: call `measure_inline(child, remaining_width, measurer)`
   b. If `child.width > remaining_width` AND `current_x > 0`:
      → break to a new line: `current_x = 0`, `current_y += line_height`,
        `line_height = 0`
   c. Position child at `(current_x, current_y)`
   d. Advance `current_x` by `child.width`
   e. Update `line_height = max(line_height, child.height)`
3. Text leaf nodes (`TextContent`) with `whiteSpace: "normal"`:
   - Split text at word boundaries
   - Place as many words as fit on the current line
   - Wrap remaining words to the next line
   - The measurer is called per word or per candidate line break

### `measure_inline`

```
measure_inline(
  node: LayoutNode,
  maxWidth: float,
  measurer: TextMeasurer
) → Size
```

For `TextContent` leaf:
- `result = measurer.measure(content.value, content.font, maxWidth)`
- Return `{ width: result.width, height: result.height }`

For inline container:
- Recursively measure its children as inline elements

### Vertical alignment within a line box

Each inline child is placed on the line box's baseline unless
`verticalAlign` specifies otherwise:

- `"baseline"` → bottom of the font's descender aligns to the line's baseline
- `"top"` → child's top edge aligns to the line box top
- `"middle"` → child's center aligns to the line's x-height midpoint
- `"bottom"` → child's bottom edge aligns to the line box bottom

Line height is the maximum of all inline children heights on that line.

### Margin collapsing

Adjacent vertical margins between block siblings collapse to the larger of
the two values (CSS margin collapsing rule):

```
collapsed_margin = max(prev_child.margin.bottom, next_child.margin.top)
```

Margins do not collapse through padding, borders, or block formatting
contexts. Simplification: this implementation collapses only sibling margins,
not parent-child margins.

### Overflow

- `overflow: "hidden"` → set `ext["paint"]["overflow"] = "hidden"` on the
  `PositionedNode`. The renderer clips to the container's bounds.
- `overflow: "scroll"` → same but renderer adds scroll affordance.
- `overflow: "visible"` → no clipping; content overflows visually.

---

## What this package does NOT do

- Does not implement `float` or `clear` — those are marked as future
- Does not implement positioned layout (`position: absolute/fixed/sticky`)
- Does not implement bidirectional text (RTL)
- Does not implement CSS columns or multi-column layout
- Does not implement `display: flex` or `display: grid` — those are
  `layout-flexbox` and `layout-grid`
- Does not perform font shaping or glyph substitution — delegates entirely
  to the `TextMeasurer`
