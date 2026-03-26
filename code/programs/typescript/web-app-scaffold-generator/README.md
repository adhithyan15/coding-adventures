# web-app-scaffold-generator

Scaffold standardized renderer-first TypeScript web apps and Electron wrappers.

## Why

This generator standardizes one architectural rule for frontend work in this
repo:

- every interactive app starts life as a renderer app
- Electron is a wrapper around that renderer, not a second UI codebase
- visualization apps use Lattice for styling so we dogfood the internal stack

That keeps the browser UI, visualization logic, and app state in one place,
while still letting us package the same renderer as a desktop app later.

## Templates

### `visualization`

Creates a browser-first Vite + React app with:

- `src/App.tsx`, `src/main.tsx`, and baseline Lattice styles
- `src/styles/installLatticeStyles.ts` to transpile `.lattice` in the browser
- `vite.config.ts` with the correct GitHub Pages base path
- `BUILD`, `README.md`, `CHANGELOG.md`, and `required_capabilities.json`
- `.github/workflows/deploy-<slug>.yml`

### `electron-wrapper`

Creates a thin Electron shell around an existing renderer app with:

- `electron/main.ts`
- `electron/tsconfig.json`
- `electron-builder.yml`
- `.github/workflows/release-<tag-prefix>.yml`

The wrapper expects a separate renderer app and packages that app’s built
`dist/` output into `renderer/` during release.

## Usage

```bash
# Browser-first renderer app
npx tsx src/index.ts code39-visualizer \
  --template visualization \
  --description "Interactive Code 39 barcode visualizer" \
  --pages-slug code39 \
  --package-deps code39,draw-instructions-svg

# Electron wrapper around an existing renderer app
npx tsx src/index.ts code39-desktop \
  --template electron-wrapper \
  --renderer-app code39-visualizer \
  --renderer-package-deps code39,draw-instructions-svg \
  --product-name "Code 39"
```

## Development

```bash
bash BUILD
```
