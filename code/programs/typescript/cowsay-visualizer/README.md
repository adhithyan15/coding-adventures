# cowsay-visualizer

A browser app that takes a message, formats it as classic Cowsay ASCII art, and renders it through the **PaintVM** abstraction — swapping the backend between **Canvas** and **SVG** without changing a line of scene-building code.

## What it demonstrates

- **PaintText in both backends** — the same `PaintText` instruction with a `canvas:` font_ref is consumed by `paint-vm-canvas` (via `ctx.fillText`) and by the new `paint-vm-svg` text handler (via SVG `<text>` element).
- **Backend portability** — `buildCowsayScene()` returns a `PaintScene` with zero knowledge of the rendering target. Calling `renderWithCanvas` or `renderWithSvg` on the same scene produces identical visual output through different code paths.
- **HiDPI canvas** — the canvas path scales the backing buffer by `devicePixelRatio` for crisp text on Retina displays.

## Architecture

```
user message string
  ↓  cowsayLines(message, wrapWidth)
string[]  (ASCII art with speech bubble + cow)
  ↓  buildCowsayScene(lines)
PaintScene  (PaintRect + PaintText per line)
  ↓  backend selected by radio button
  ├─ renderWithCanvas(scene, container)
  │    createCanvasVM().execute(scene, ctx)  →  <canvas>
  └─ renderWithSvg(scene, container)
       renderToSvgString(scene)  →  <div> innerHTML
```

## Running locally

```bash
cd code/programs/typescript/cowsay-visualizer
npm install
npm run dev
# → http://localhost:5175
```

## Font ref scheme

`buildCowsayScene` uses the `canvas:` font_ref format defined in spec TXT03d:

```
canvas:<family>@<size>[:<weight>[:<style>]]
```

For example: `canvas:Courier New@15`

- **paint-vm-canvas** parses this into a CSS font shorthand for `ctx.font`
- **paint-vm-svg** parses this into `font-family`, `font-size`, and optionally `font-weight` / `font-style` SVG presentation attributes

Both backends understand the same scheme — no special-casing in the scene builder.
