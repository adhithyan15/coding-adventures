# VisiCalc — Mosaic reference application

VisiCalc currently compiles its Grid and FormulaBar from shared Mosaic sources
and uses the Rust spreadsheet engine through a committed WASM bundle. The React
host still owns selection, edit buffers and the row window. Migration to one
UI38 Rust `MosaicApp` adapter and generated application hosts is tracked in
[#14267](https://github.com/adhithyan15/coding-adventures/issues/14267); the
[investigation](../../../specs/visicalc-mosaic-investigation-20260905.md) records
current architecture and validation limits.

The [Rust MosaicApp adapter](../../../packages/rust/visicalc-mosaic-app) now
passes the shared presentation contract and exports the standard native ABI.
This React host still uses its existing reducer and engine binding; switching
it to the adapter is tracked in #14272.

## Build and run

Requires Node.js, npm, Rust/Cargo and Bash. Linux compiler builds also require
Cairo development libraries (`libcairo2-dev` on Ubuntu).

```bash
cd code/programs/typescript/visicalc
npm ci
npm test
npm run build
npm run dev
```

Tests and build regenerate the Mosaic components. The production build checks
TypeScript application and test sources before Vite bundles the app. The
[VisiCalc workflow](../../../../.github/workflows/visicalc.yml) executes tests and
build on Linux and Windows. Repository package discovery and CodeQL pull-request
coverage still need integration under #14270/#14285.

## Source ownership

- [Shared Mosaic sources](../../mosaic/visicalc): `.mil` interfaces, `.mll`
  layouts, `.msl` styles and versioned presentation fixtures. Grid composition
  delegates to `mosaic-pkg-grid::Grid`; it is not the old compiler Grid primitive.
- [Build script](scripts/build.sh): compiles the components into the ignored
  `src/components/` directory and copies the committed engine bundle from
  `../visicalc-html/vendor/` into ignored `public/`. Do not edit generated files.
- [Engine binding](src/app/engine.ts): loads and seeds the real Rust/WASM
  workbook, returns raw formula source and computed display windows, and commits
  changes through the engine. Calculation belongs to
  [spreadsheet-core](../../../packages/rust/spreadsheet-core).
- [Host](src/app/App.tsx) and [reducer](src/app/state.ts): temporary presentation
  logic. Workbook coordinates are absolute and zero-based; only the generated
  Grid boundary translates row indices relative to the rendered slice.

Nested `list<list<text>>` props and exported generated event unions work today.
The grid renders computed results; the formula bar shows source or the active
edit buffer. Formula-bar commit preserves selection; inline commit moves down.

## Shared acceptance

The [presentation contract](../../mosaic/visicalc/fixtures/README.md) defines a
shared seed and 16 ordered event/expectation steps. The React replay drives real
generated controls and verifies raw source, computed values, edit state,
selection, and the rendered row slice. The same fixture is intended for the
Rust adapter and native acceptance runners. It supplements focused regressions
for clearing cells, text-input keys and coordinate bounds.

Passing this baseline is not completion of the application migration. The fixed
30-row slice can extend below a short window; physical scrolling, responsive
sizing and sticky headers remain in #14277. The outstanding backlog also covers
save/reopen, accessible focus and announcements, exceptional shared design,
native launch verification and downloadable GitHub Releases (#14282).

The [Electron shell](../visicalc-electron) loads this app's production output.
Native packaging and release acceptance are tracked separately from web builds.
