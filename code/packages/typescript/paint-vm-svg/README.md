# @coding-adventures/paint-vm-svg

SVG string backend for PaintVM (P2D02).

Renders a `PaintScene` to a complete `<svg>` string without touching the DOM.
Works in any JavaScript environment: browser, Node.js, Deno, Bun, Cloudflare
Workers. Output can be sent as `image/svg+xml`, embedded in HTML via
`innerHTML`, saved as a `.svg` file, or piped into a headless renderer.

## Where it fits

```
PaintScene
    │
    ▼
PaintVM<SvgContext>          ← this package
    │
    └── renderToSvgString(scene) → "<svg>...</svg>"
                                        │
                                        ├── innerHTML = svgString   (browser)
                                        ├── fs.writeFile("out.svg") (Node.js)
                                        └── res.send(svgString)     (HTTP)
```

## Why SVG over Canvas for server-side rendering?

SVG is resolution-independent vector output: one render is valid at any DPI.
Canvas produces raster pixels at a fixed resolution. For:

- Static diagrams, charts, icons — SVG is smaller and scales perfectly
- PDF generation (via headless Chromium or Cairo)
- Email HTML with embedded images
- SEO-indexable charts (SVG is text; search engines can index it)

For 60fps interactive rendering or pixel-accurate output, use
`@coding-adventures/paint-vm-canvas` instead.

## Installation

```sh
npm install @coding-adventures/paint-vm-svg
```

## Usage

### Simple render

```typescript
import { renderToSvgString } from "@coding-adventures/paint-vm-svg";
import { paintScene, paintRect, paintEllipse } from "@coding-adventures/paint-instructions";

const svg = renderToSvgString(
  paintScene(800, 400, "#ffffff", [
    paintRect(20, 20, 200, 100, { fill: "#3b82f6", corner_radius: 8 }),
    paintEllipse(400, 200, 80, 80, { fill: "#ef4444", stroke: "#b91c1c", stroke_width: 2 }),
  ]),
);

// Browser
document.getElementById("chart")!.innerHTML = svg;

// Node.js HTTP response
res.setHeader("Content-Type", "image/svg+xml");
res.send(svg);
```

### Gradients

```typescript
import { paintGradient } from "@coding-adventures/paint-instructions";

const svg = renderToSvgString(
  paintScene(400, 200, "transparent", [
    paintGradient("linear",
      [{ offset: 0, color: "#3b82f6" }, { offset: 1, color: "#8b5cf6" }],
      { id: "sky", x1: 0, y1: 0, x2: 400, y2: 0 },
    ),
    paintRect(0, 0, 400, 200, { fill: "url(#sky)" }),
  ]),
);
```

### Filters and layers

```typescript
const svg = renderToSvgString(
  paintScene(400, 300, "#ffffff", [
    paintLayer(
      [paintRect(50, 50, 200, 100, { fill: "#3b82f6" })],
      {
        filters: [
          { kind: "drop_shadow", dx: 4, dy: 4, blur: 8, color: "#00000040" },
          { kind: "blur", radius: 2 },
        ],
        blend_mode: "multiply",
        opacity: 0.9,
      },
    ),
  ]),
);
```

### Low-level API

```typescript
import { createSvgVM, makeSvgContext, assembleSvg } from "@coding-adventures/paint-vm-svg";

const vm = createSvgVM();
const ctx = makeSvgContext();
vm.execute(scene, ctx);
// Inspect ctx.defs and ctx.elements before assembling
const svg = assembleSvg(scene, ctx);
```

## Security

This package generates SVG output intended for browser rendering and
server-side headless renderers. It applies the following security controls:

- **Numeric injection prevention** — every numeric value (coordinates, radii,
  font sizes, opacity, filter parameters, transform matrix values) is validated
  with `safeNum()` before SVG attribute interpolation. Non-finite values
  (`NaN`, `Infinity`) throw a `RangeError`.
- **Attribute allowlists** — `fill_rule`, `stroke_cap`, `stroke_join`, and
  `blend_mode` are checked against runtime allowlists. Unknown values fall back
  to safe defaults.
- **Glyph ID safety** — `glyph_id` values are range-validated to the Unicode
  codepoint range `[0, 0x10FFFF]`. Out-of-range values are replaced with
  U+FFFD (the replacement character).
- **Image URI validation** — `PaintImage.src` URIs are validated to allow
  only `data:`, `https:`, and `http:` schemes. `javascript:`, `file:`, and
  other dangerous schemes are replaced with a safe empty placeholder.

## API

### `renderToSvgString(scene: PaintScene): string`

Render a `PaintScene` to a complete SVG string. The primary entry point.

### `createSvgVM(): PaintVM<SvgContext>`

Returns a VM with all 10 instruction handlers registered.

### `makeSvgContext(): SvgContext`

Returns a fresh context `{ defs: [], elements: [], clipCounter: 0, filterCounter: 0 }`.

### `assembleSvg(scene: PaintScene, ctx: SvgContext): string`

Assembles `defs + elements` into a final `<svg>` string.

## Related packages

| Package | Role |
|---|---|
| `@coding-adventures/paint-instructions` | Instruction types and builder helpers |
| `@coding-adventures/paint-vm` | Generic VM engine (dispatch table, patch diff) |
| `@coding-adventures/paint-vm-svg` | SVG string backend (this package) |
| `@coding-adventures/paint-vm-canvas` | Canvas 2D backend |

## Spec

[P2D02 — Paint VM SVG](../../../specs/P2D02-paint-vm-svg.md)
