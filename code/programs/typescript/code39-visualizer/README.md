# code39-visualizer

`code39-visualizer` is a renderer-first web app for exploring the Code 39 pipeline.

The app does not call a barcode image library directly. Instead, it walks through the same layered architecture used elsewhere in the repository:

1. `@coding-adventures/code39` validates and encodes the input.
2. The barcode package turns the encoding into backend-neutral draw instructions.
3. `@coding-adventures/draw-instructions-svg` serializes that draw scene into SVG.
4. `@coding-adventures/lattice-transpiler` compiles the app's `.lattice` styles into CSS in the browser.
5. The React app renders the SVG and exposes the intermediate structures so the barcode stays explainable.

That split matters because this app is meant to be a standardized renderer app first. An Electron desktop build can wrap it later without duplicating the barcode UI.

## What The App Shows

- A text input for any standard Code 39 value
- The normalized value after lowercase characters are promoted to uppercase
- The encoded character stream, including start and stop markers
- The expanded run stream of narrow and wide bars and spaces
- The final SVG barcode preview
- A Lattice-authored style layer that gets transpiled at startup

## Development

```bash
bash BUILD
cd code/programs/typescript/code39-visualizer
npm run dev
```

## Why This App Exists

This program is also the first consumer of the new TypeScript web app scaffold generator. The generator standardizes:

- renderer-first Vite + React app structure
- local package dependency wiring
- GitHub Pages deployment workflow
- the path toward an Electron wrapper generated as a separate app shell
