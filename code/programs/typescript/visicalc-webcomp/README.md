# VisiCalc — WebComponent demo (live)

A **working** VisiCalc built from real Web Components, computing live on the
Rust `spreadsheet-core` engine **compiled to WebAssembly** — the same `.wasm`
module the [HTML demo](../visicalc-html) runs. The companion web backend to the
HTML one: same engine, different UI technology.

## What it shows

A cross-footing budget (column E totals each row, row 5 totals each column,
E5 is the grand total — all formulas). Click a cell to select it; the formula
bar shows its source. Type a value or a formula (e.g. `=SUM(A1:A4)`) in the
formula bar and press Enter, or double-click a cell to edit in place — every
dependent cell recomputes.

## How it works

Two independently-generated halves, glued by host code that owns *no*
spreadsheet logic:

1. **The UI** — two real custom elements compiled by
   `mosaic-compile --backend webcomponent` from `code/programs/mosaic/visicalc/*` (the
   same source the React and HTML demos consume):
   - `<mos-grid>` — a shadow-DOM table rendered from its observed attributes
     (`viewport-rows`, `selected-row/col`, `edit-row/col`, …).
   - `<mos-formula-bar>` — a shadow-DOM input that fires `mosaic:commit` /
     `mosaic:formulaChange` / `mosaic:cancel` CustomEvents.

   Both are committed under `vendor/` (regenerate with `bash scripts/build.sh`,
   which writes `build/`, then copy into `vendor/`).

2. **The engine** — the Rust `spreadsheet-core` engine compiled to `wasm32`,
   loaded by `vendor/spreadsheet-engine-wasm.js` (the `.wasm` embedded as
   base64, so the page opens from `file://`). Regenerate with
   `bash scripts/bundle-wasm-engine.sh`.

The inline glue awaits the WASM module, seeds a workbook, pushes the workbook's
*computed* values into the `<mos-grid>` attributes, and feeds user edits
(formula-bar commit, inline-cell edit, cell-click selection) back through
`workbook.setCell`. `<mos-grid>` cells carry no click handler, so selection is
wired by reaching into the element's shadow DOM — the engine stays the single
source of truth.

```
index.html
  ├─ <script src="vendor/spreadsheet-engine-wasm.js">  ← Rust engine, as WASM
  ├─ import vendor/{Grid,FormulaBar}.js                ← mosaic-compiled elements
  └─ <script type="module"> (inline)                  ← thin glue: model → attrs → edit
```

## How to view

```
open code/programs/typescript/visicalc-webcomp/index.html
```

Any modern browser works — no server. The events from `<mos-formula-bar>` /
`<mos-grid>` are also logged at the bottom of the page.
