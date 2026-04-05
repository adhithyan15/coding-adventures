# @coding-adventures/layout-text-measure-canvas

**Accurate browser Canvas text measurer** using `CanvasRenderingContext2D.measureText()`.

TypeScript/browser only (or Node.js with the `canvas` npm package).

## Usage

```ts
import { createCanvasMeasurer } from "@coding-adventures/layout-text-measure-canvas";
import { font_spec } from "@coding-adventures/layout-ir";

// Wait for fonts to load before measuring
await document.fonts.ready;

const canvas = document.createElement("canvas");
const ctx = canvas.getContext("2d")!;
const measurer = createCanvasMeasurer(ctx);

const font = font_spec("Inter", 16);

// Single line
const r1 = measurer.measure("Hello world", font, null);
// r1.width  — actual browser-measured pixel width
// r1.height — actual bounding box height
// r1.lineCount = 1

// Multi-line with maxWidth
const r2 = measurer.measure("A longer paragraph of text...", font, 300);
// r2.lineCount — word-wrapped line count
// r2.height    — lineCount × font.size × lineHeight
```

## Font loading

`measureText` returns inaccurate results if fonts haven't loaded. Always
create the measurer after `await document.fonts.ready`.

## fontSpecToCss

The `fontSpecToCss` helper converts a `FontSpec` to a CSS font string:

```ts
import { fontSpecToCss } from "@coding-adventures/layout-text-measure-canvas";
import { font_spec, font_bold } from "@coding-adventures/layout-ir";

fontSpecToCss(font_spec("Arial", 16))     // "400 16px 'Arial'"
fontSpecToCss(font_bold(font_spec(...)))  // "700 16px 'Arial'"
```

## See also

- [UI09 — layout-text-measure spec](../../specs/UI09-layout-text-measure.md)
- `layout-text-measure-estimated` — fast zero-dep measurer for CI/tests
- `layout-text-measure-rs` — accurate Rust+fontdue measurer via FFI
