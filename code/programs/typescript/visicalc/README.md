# VisiCalc — Mosaic reference application

The web app renders the root `VisiCalc` Mosaic package and runs the standard
Rust `visicalc-mosaic-app` through `mosaic-app-wasm`. Rust owns workbook data,
formula evaluation, selection, edit buffers and row-window coordinates. The
React host loads/disposes the runtime, maps browser input to semantic events,
and renders returned props and announcements. It contains no workbook reducer.

## Build and run

Requires Node.js, npm, Rust/Cargo, rustup and Bash. Linux compiler builds also
require Cairo development libraries (`libcairo2-dev` on Ubuntu).

```bash
cd code/programs/typescript/visicalc
npm ci
npm test
npm run build
npm run dev
```

The build installs the wasm32 Rust target, compiles the real application,
emits both root themes through `mosaic-compile pkg`, and bundles the WASM asset.
Generated components and WASM are ignored; do not edit them. Production builds
check application and test TypeScript before Vite bundles the app.

## Source ownership

- [Root Mosaic package](../../mosaic/visicalc): manifest, root interface/layout,
  shared light/dark styles and versioned workflow fixtures. The grid comes from
  `mosaic-pkg-grid::Grid`.
- [Rust adapter](../../../packages/rust/visicalc-mosaic-app): application logic
  over spreadsheet-core, standard native ABI and WASM lifecycle exports.
- [Standard WASM host](../../../packages/rust/mosaic-app-wasm): transport,
  event sequencing, snapshots and independent runtime ownership.
- [React host](src/app/App.tsx): startup, browser input and generated view wiring.
  Generated grid events remain relative to the slice; the Rust adapter translates
  them to absolute workbook coordinates. Formula commit retains selection and
  inline commit advances one row.

## Acceptance and remaining work

Nine web tests run generated controls against the actual Rust WASM application.
The shared 16-step fixture checks selection, edits, displayed rows and committed
source/calculated values through independent snapshot restores. Focused tests
cover clearing cells, input keys, viewport edges and invalid request rejection.
The Linux/Windows [workflow](../../../../.github/workflows/visicalc.yml) regenerates
and tests the app and production build. The root package also has a Cargo source
compilation harness and BUILD entry.

The ongoing [migration backlog](https://github.com/adhithyan15/coding-adventures/issues/14267)
requires responsive physical scrolling/sticky headers (#14277), accessible focus
and announcements (#14278), full save/reopen and workbook commands (#14279),
exceptional shared design (#14273), generated native acceptance and downloadable
GitHub Releases (#14282). The current 30-row slice can exceed a short window.
The runtime supports opaque snapshots; durable web storage and file controls
are not connected yet. Repository/CodeQL source discovery remains under #14270.

The [Electron shell](../visicalc-electron) loads this app's production output.
Native packaging and downloaded-release acceptance remain separate deliverables.
