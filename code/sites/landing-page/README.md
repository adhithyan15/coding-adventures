# Coding Adventures landing page

The repository root is a seven-stage Forme site, not a hand-maintained output
file. Its declarative content lives in `data/index.landing`; `parse-landing.ts`
validates that source into Content IR, and `forme.config.ts` wires asset
identity, routing, rendering, loading, fingerprinting, and emission into one
typed DAG.

```text
forme-source-fs → site-landing-parse → forme-resolve-asset-refs-fs → forme-router
                                                             ├→ site-landing-render ─┐
                                                             └→ forme-load-assets-fs ┴→ forme-emit-site-fs
```

## Build

```bash
cd code/sites/landing-page
npm run build:clean
```

The clean build bootstraps every local `file:` dependency, validates the
content model, compiles the portable typography/color layer through Style IR
and the AOT slicer, inlines the browser-specific layout layer, fingerprints the
social image, rewrites its metadata URL for the GitHub Pages project prefix,
and writes `dist/index.html` plus `dist/assets/`.

Run `npm test` for the content-schema, renderer, and bootstrap contracts.

## Editing the page

- `data/index.landing` owns copy, navigation, cards, repository statistics,
  Forme status, and links.
- `landing-style.ts` owns backend-portable style tokens and rules.
- `landing.css` owns web-only layout primitives that the current
  backend-neutral Style IR does not model, including grid, flex, pseudo
  elements, and responsive breakpoints.
- `parse-landing.ts` and `render-landing.ts` are the site adapter boundary. The
  remaining source, route, asset, and emitter stages are reusable Forme
  packages.

Do not edit generated files under `dist/`; the Pages workflow always rebuilds
from a clean checkout before publishing.
