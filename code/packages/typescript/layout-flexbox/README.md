# @coding-adventures/layout-flexbox

**CSS Flexbox layout algorithm** for the coding-adventures layout pipeline.

Pure layout — no rendering, no fonts, no I/O.

## Usage

```ts
import { layout_flexbox } from "@coding-adventures/layout-flexbox";
import { createEstimatedMeasurer } from "@coding-adventures/layout-text-measure-estimated";
import {
  container, leaf_text, size_fill, size_wrap,
  font_spec, rgb, edges_all, constraints_width
} from "@coding-adventures/layout-ir";

const measurer = createEstimatedMeasurer();

const tree = container(
  [
    leaf_text({ kind: "text", value: "Hello",
                font: font_spec("Arial", 16), color: rgb(0,0,0),
                maxLines: null, textAlign: "start" }),
    leaf_text({ kind: "text", value: "World",
                font: font_spec("Arial", 16), color: rgb(0,0,0),
                maxLines: null, textAlign: "start" }),
  ],
  {
    width: size_fill(),
    height: size_wrap(),
    padding: edges_all(16),
    ext: { flex: { direction: "row", gap: 8 } },
  }
);

const result = layout_flexbox(tree, constraints_width(800), measurer);
// result.width = 800
// result.children[0] = { x: 16, y: 16, width: ..., height: ... }
// result.children[1] = { x: 16 + w0 + 8, y: 16, ... }
```

## flex ext schema

```ts
ext: {
  flex: {
    // Container properties
    direction?: "row" | "column"          // default "column"
    wrap?: "nowrap" | "wrap"              // default "nowrap"
    alignItems?: "start"|"center"|"end"|"stretch"       // default "stretch"
    justifyContent?: "start"|"center"|"end"|"between"|"around"|"evenly"
    gap?: number                           // between items
    rowGap?: number                        // between rows (wrap only)
    columnGap?: number                     // between columns

    // Item properties (on child nodes)
    grow?: number                          // flex-grow, default 0
    shrink?: number                        // flex-shrink, default 1
    basis?: SizeValue | null               // flex-basis, default null
    alignSelf?: "start"|"center"|"end"|"stretch"|"auto"
    order?: number                         // default 0
  }
}
```

## See also

- [UI03 — layout-flexbox spec](../../specs/UI03-layout-flexbox.md)
- `layout-ir` — core types
- `layout-block` — block/inline flow layout
- `layout-grid` — CSS Grid layout
