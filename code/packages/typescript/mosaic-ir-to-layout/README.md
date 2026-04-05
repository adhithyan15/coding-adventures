# mosaic-ir-to-layout

Converts a **MosaicIR** component tree (produced by `mosaic-analyzer`) into a
**LayoutNode** tree suitable for flexbox layout (`layout-flexbox`).

This is the bridge between the Mosaic compiler's semantic layer and the
rendering pipeline.

---

## What is MosaicIR?

`mosaic-analyzer` validates a parsed Mosaic AST and emits a typed IR.
Each node in the IR describes a UI primitive (`Column`, `Row`, `Text`, …)
together with its properties and typed slot declarations.

## What is a LayoutNode?

`layout-ir` defines the data structure that flexbox and paint algorithms
consume. It holds dimensions, flex-container/flex-item extensions, paint
extensions (background, border, shadow), font info, and child lists.

---

## Position in the pipeline

```
.mosaic source
      │
 mosaic-lexer        — tokenise
      │
 mosaic-parser       — parse → ASTNode
      │
 mosaic-analyzer     — validate → MosaicComponent (IR)
      │
 mosaic-ir-to-layout — ← YOU ARE HERE
      │
 layout-flexbox      — position nodes
      │
 layout-to-paint     — emit paint instructions
```

---

## Installation

```bash
npm install coding-adventures-mosaic-ir-to-layout
```

---

## Usage

```typescript
import {
  mosaic_ir_to_layout,
  mosaic_default_theme,
  type SlotMap,
} from "coding-adventures-mosaic-ir-to-layout";
import { analyzeMosaic } from "coding-adventures-mosaic-analyzer";
import { parseMosaic } from "coding-adventures-mosaic-parser";
import { tokenizeMosaic } from "coding-adventures-mosaic-lexer";

const tokens = tokenizeMosaic(source);
const ast    = parseMosaic(tokens);
const ir     = analyzeMosaic(ast);

// Runtime slot values supplied by the caller
const slots: SlotMap = new Map([
  ["title",  "Hello, world!"],
  ["count",  42],
  ["active", true],
]);

const rootNode = mosaic_ir_to_layout(ir, slots, mosaic_default_theme());
```

---

## API

### `mosaic_ir_to_layout(component, slots, theme)`

| Parameter   | Type                | Description                         |
|-------------|---------------------|-------------------------------------|
| `component` | `MosaicComponent`   | Typed IR from `mosaic-analyzer`     |
| `slots`     | `SlotMap`           | Runtime slot values                 |
| `theme`     | `MosaicLayoutTheme` | Default font, text colour, base size|

Returns a `LayoutNode` — the root of the layout tree.

### `mosaic_default_theme()`

Returns a sensible theme:

```typescript
{
  defaultFont: { family: "system-ui", size: 16, weight: 400, style: "normal" },
  defaultTextColor: { r: 0, g: 0, b: 0, a: 255 },
  baseFontSize: 16,
}
```

### `SlotValue`

```typescript
type SlotValue =
  | string
  | number
  | boolean
  | Color                 // { r, g, b, a }
  | LayoutNode
  | SlotValue[];          // list — used by each
```

### `SlotMap`

`Map<string, SlotValue>` — keys are slot names declared in the component.

---

## Primitive node mappings

| Mosaic tag  | LayoutNode shape                        |
|-------------|----------------------------------------|
| `Column`    | flex container, `direction: "column"`  |
| `Row`       | flex container, `direction: "row"`     |
| `Box`       | flex container, `direction: "column"`  |
| `Text`      | leaf, `content: { kind: "text" }`      |
| `Image`     | leaf, `content: { kind: "image" }`     |
| `Spacer`    | flexible spacer, `flexGrow: 1`         |
| `Divider`   | thin separator (1 px height or width)  |
| `Scroll`    | container, `overflow: "scroll"`        |

---

## Property resolution

### Dimensions

`width` and `height` accept:
- a number literal → fixed pixels
- `"fill"` ident → `{ kind: "fill" }`
- `"shrink"` ident → `{ kind: "shrink" }`
- a `dimension` value → pixel number

### Visual properties → PaintExt

| Property         | Effect                              |
|-----------------|-------------------------------------|
| `background`    | `backgroundColor` (hex colour)      |
| `border-color`  | `borderColor` (hex colour)          |
| `border-width`  | `borderWidth` (px number)           |
| `corner-radius` | `cornerRadius` (px number)          |
| `opacity`       | `opacity` (0–1)                     |
| `shadow`        | elevation table: none/low/medium/high |

### Font properties

| Property      | Effect                        |
|---------------|-------------------------------|
| `style`       | shorthand: `body`/`heading`/`large`/`caption` |
| `font-size`   | override font size (px)       |
| `font-weight` | override font weight (400/700) |
| `font-style`  | override italic/normal        |

---

## `when` and `each` children

**`when`** children render only if the referenced slot is truthy:

```mosaic
when items {
  Text { content: "Has items" }
}
```

**`each`** children render once per element in a list slot, with the loop
variable accessible via `slot_ref`:

```mosaic
each items as item {
  Text { content: item }
}
```

---

## Colour format

All hex strings (`#rgb`, `#rrggbb`, `#rrggbbaa`) are parsed into
`Color { r, g, b, a }` (0–255 channels).  Invalid strings fall back to
fully-transparent black.
