# @coding-adventures/layout-block

Block and inline flow layout algorithm — the layout model underlying HTML/CSS
normal flow (block formatting context).

This is the layout engine for document renderers: Markdown, rich text, email,
and any content modelled as a tree of block and inline nodes.

```
LayoutNode[] + Constraints + TextMeasurer
    ↓  layout_block()
PositionedNode[]  →  layout-to-paint  →  paint-vm-canvas
```

See: `code/specs/UI07-layout-block.md`

## Installation

```bash
npm install @coding-adventures/layout-block
```

## Usage

```ts
import { layout_block } from "@coding-adventures/layout-block";
import { constraints_width } from "@coding-adventures/layout-ir";

const result = layout_block(rootNode, constraints_width(800), measurer);
// result is a fully-positioned PositionedNode tree
```

## Block/Inline Model

Nodes participate in layout based on `ext["block"].display`:

| Value | Behaviour |
|-------|-----------|
| `"block"` (default) | Full-width box; stacks vertically below siblings |
| `"inline"` | Inline box; flows horizontally, wraps at container edge |

Mixed block/inline siblings are handled via **anonymous block generation** —
inline siblings are automatically grouped into an anonymous block container
before layout begins, following the CSS anonymous box rule.

## API

### `layout_block(container, constraints, measurer) → PositionedNode`

| Parameter | Type | Description |
|-----------|------|-------------|
| `container` | `LayoutNode` | The block container to lay out |
| `constraints` | `Constraints` | Parent size constraints (usually `constraints_width(w)`) |
| `measurer` | `TextMeasurer` | Text measurement provider |

### `BlockExt`

Attach block/inline metadata to any `LayoutNode` via `ext["block"]`:

```ts
const para: LayoutNode = {
  // ...
  ext: {
    block: {
      display: "block",
      overflow: "hidden",
      whiteSpace: "normal",
      paragraphSpacing: 16,
    },
  },
};
```

| Property | Type | Default | Effect |
|----------|------|---------|--------|
| `display` | `"block" \| "inline"` | `"block"` | Block vs inline formatting |
| `overflow` | `"visible" \| "hidden" \| "scroll"` | `"visible"` | Sets `ext["paint"].overflow` |
| `whiteSpace` | `"normal" \| "pre" \| "nowrap"` | `"normal"` | Word wrapping mode |
| `wordBreak` | `"normal" \| "break-all"` | `"normal"` | Character-level break (future) |
| `verticalAlign` | `"baseline" \| "top" \| "middle" \| "bottom"` | `"baseline"` | Inline vertical alignment |
| `paragraphSpacing` | `number` | `0` | Extra gap after a block child |

## Margin Collapsing

Adjacent block siblings have their vertical margins collapsed:

```
collapsed = max(child1.margin.bottom, child2.margin.top)
```

This is the CSS margin collapsing rule for siblings. Parent-child collapsing
is not implemented (simplified model).

## Related Packages

- `@coding-adventures/layout-ir` — shared types (`LayoutNode`, `PositionedNode`, etc.)
- `@coding-adventures/layout-flexbox` — CSS Flexbox algorithm
- `@coding-adventures/layout-grid` — CSS Grid algorithm
- `@coding-adventures/layout-to-paint` — converts positioned nodes to paint instructions
- `@coding-adventures/document-ast-to-layout` — converts DocumentAST to LayoutNode trees
