# VisiCalc — HTML demo (live)

A **working** VisiCalc in a single `index.html` you can open from disk:
a formula bar above a 5×5 grid that actually computes. It is VisiCalc
built on top of the Rust
[`spreadsheet-core`](../../code/packages/rust/spreadsheet-core) engine
**compiled to WebAssembly** — the UI shell comes from the Mosaic
pipeline, the *behaviour* comes from the engine. (The same engine, via
the same WASM module, also backs the WebComponent demo; native demos use
it through a C ABI.)

> **Also here: [`infinite.html`](infinite.html) — a virtualized, effectively
> *infinite* sheet.** Where `index.html` is the fixed 5×5 parity grid, this page
> proves the engine's *viewport primitive*: the sheet is u32 × u32 (~4.3 billion
> each way) and sparse, and only the **visible window** of cells is ever in the
> DOM. Scroll thousands of rows / dozens of columns and the on-page "DOM cells
> materialized" counter stays small. It's driven entirely by the engine's
> `getDisplayWindow` (the visible rectangle as ready-to-paint **display
> strings** — each cell already rendered through its Excel-style format code by
> the engine, e.g. `1234.5` with `"#,##0.00"` → `"1,234.50"`, so the page never
> re-derives number formatting), `setFormat` (attach a format code),
> `usedRange` (scrollbar sizing), `columnLetters` (A…Z, AA…), `changedSince`
> (re-fetch only what an edit dirtied), and `fill` (drag-fill). The seeded sheet
> formats its cross-foot totals with thousands grouping + two decimals and
> renders the far-flung `Z1000` total as a percent, so the formatting is visible
> while scrolling. The **"Fill ↓ 10"** button replicates the selected cell into
> the 10 rows below it — the engine shifts each copy's relative references
> (`=A1`→`=A2`, …), pins absolute (`$`) refs, and carries the format, in one
> `workbook.fill` call. The **Copy / Cut / Paste** buttons drive the engine's
> clipboard (`workbook.copy`/`cut`/`paste`): copy the selected cell, then paste
> it elsewhere with its relative references shifted by the destination's offset
> (absolute `$` refs pinned, format carried); a cut clears the source on paste.
> The **Save / Load** buttons serialize the whole workbook (`workbook.serialize`)
> to a single JSON document in the browser's `localStorage` and restore it
> (`workbook.deserialize`): the document captures only the *source* (formula text
> + typed literals) and per-cell formats — not the computed values, which the
> engine recomputes on load, so a loaded formula stays live.
> The **Undo / Redo** buttons walk the engine's snapshot history
> (`workbook.undo`/`redo`, gated by `canUndo`/`canRedo`): every edit is
> reversible, a restored formula recomputes live, and a fresh edit forks history
> (drops the redo branch).
> The **+ Row / − Row / + Col / − Col** buttons are **structural edits**
> (`workbook.insertRows`/`deleteRows`/`insertCols`/`deleteCols`): insert or
> delete the selected cell's row/column, and the engine shifts every formula
> reference at or after the band so dependents keep pointing at their precedents
> (`=A1+A2` with a row inserted above becomes `=A1+A3`); a reference whose whole
> band is deleted becomes `#REF!`.
> The **.00 / % / $ / Gen** buttons apply a number **format** to the selected cell
> (`workbook.setFormat` with an Excel-style code: `#,##0.00`, `0.0%`,
> `$#,##0.00`, or `""` to clear). The format is display-only — the engine renders
> the stored value through the code (`getDisplayWindow`), so `15` shows as
> `15.00` / `1500.0%` / `$15.00` without changing the underlying number.
> **Resize columns and rows by dragging** a column header's right edge or a row
> number's bottom edge (double-click the handle to auto-fit back to the default).
> The size lives in the engine
> (`workbook.columnWidth`/`setColumnWidth`/`rowHeight`/`setRowHeight`, with bulk
> `columnWidths`/`rowHeights` for a one-call viewport read) — so it **persists
> through Save / Load**, **shifts with its column / row** on an insert/delete
> (widen C, insert a column at B, and the now-D column stays wide), and a whole
> drag is a **single Undo** step. The seed opens with a wide column C (140 px) and
> a tall row 2 (40 px) so the non-uniform grid is visible immediately. The
> virtualized window math uses exact per-column / per-row cumulative offsets (a
> prefix-sum over the bounded extent), not a uniform-cell assumption.
> Headless proof: `node scripts/verify-infinite.mjs` replays the exact windowing
> math against the committed WASM bundle and asserts the render stays bounded,
> the formatted display strings are correct, a formula 1000 rows down is
> reachable, the gaps are empty (sparse), an edit's diff reaches the far cell
> that depends on it, a save → mutate → load round trip restores the
> workbook (formulas recompute live; garbage input is rejected), an
> undo/redo walk reverses two edits then replays them with the formula live, an
> insert/delete row & column round trip shifts a formula's references (and
> turns a reference into `#REF!` when its row is deleted), and a column-width /
> row-height round trip sets a size, reads it back, rejects a bad value, persists
> it through save/load, and shifts it when a column is inserted before it.

## Design language

`infinite.html` is the **reference UI** for the VisiCalc demos — the visual
language the native demos (Qt/Flutter/Compose/XAML/SwiftUI) mirror. It keeps a
dark, modern-spreadsheet look, defined by a small set of CSS tokens in the
page's `:root` so the whole surface reads as one considered panel:

- **Palette + scale** — one set of custom properties (`--bg`/`--panel`/
  `--surface`/`--line`/`--ink`/`--muted`/`--accent`, radii, a mono + a UI font)
  rather than scattered hex values. The chrome uses the UI font; cells and the
  formula field use the mono font.
- **Toolbar** — an address **pill**, an `fx` marker, then a grown formula field
  with an accent **focus ring**; actions are **segmented button groups**
  (drag-fill · clipboard · file · structure · format · history) separated by thin rules,
  each with hover/active/disabled states.
- **Grid** — subtle **zebra** row banding, a 2-px **accent selection ring**, and
  the selected cell's **row + column headers tint to the accent** so the
  cursor's position reads at a glance.
- **Status line** — a footer, hairline-separated, showing the live virtual-grid
  size, materialized-cell count, and revision.

## What it shows

A cross-footing budget: columns A–D hold numbers, column E totals each
row (`=SUM(A1:D1)` …), row 5 totals each column, and E5 is the grand
total — so it cross-foots both ways.

- **Click** a cell to select it; the formula bar shows its raw
  contents (a number, or the underlying formula for a computed cell).
- **Type** a value or a formula (e.g. `=SUM(A1:A4)`, `=A1*2`,
  `=AVERAGE(B1:B4)`) into the formula bar and press **Enter** — or
  **double-click** a cell to edit it in place.
- Every dependent cell recomputes. Errors propagate too:
  `=1/0` shows `#DIV/0!`, and any total that depends on it does as well.

## How it works

Two independently-generated halves, glued by ~150 lines of host code
that own *no* spreadsheet logic:

1. **The shell** — the FormulaBar and Grid markup are generated by
   `mosaic-compile --backend html` from
   `code/programs/mosaic/visicalc/{FormulaBar,Grid}.{mil,mll,msl}` (the same
   sources the React, SwiftUI, Qt, Flutter and Compose demos consume).
   The Grid is emitted as a static template with `mosaic-for` /
   `mosaic-if` markers and `{{…}}` placeholders; a small inline
   template-expander hydrates it. Regenerate with `bash scripts/build.sh`.

2. **The engine** — the Rust `spreadsheet-core` engine (cells +
   dependency graph + incremental topological recalc + ~50 functions
   delegated to the Layer-1 cores) compiled to `wasm32`. The browser
   loader `vendor/spreadsheet-engine-wasm.js` embeds the `.wasm` as
   base64 and, once instantiated, exposes `window.SpreadsheetEngine`
   (same API as the TypeScript engine). Regenerate with
   `bash scripts/bundle-wasm-engine.sh`.

The inline `<script>` in `index.html` is the glue: it awaits the WASM
module, seeds a workbook, renders the workbook's *computed* values
through the compiled Grid template on every change, and feeds user edits
back into `workbook.setCell(addr, raw)`. No framework — and because the
`.wasm` is embedded (not fetched), the page still opens directly via
`file://`.

```
index.html
  ├─ <script src="vendor/spreadsheet-engine-wasm.js">  ← Rust engine, as WASM
  ├─ <template id="grid-template">                     ← mosaic-compiled UI shell
  └─ <script> (inline)                                 ← thin glue: model → render → edit
```

## How to view

```
open code/programs/typescript/visicalc-html/index.html
```

Any modern browser works — no module loader, no server. To regenerate
the artifacts after changing sources:

```bash
bash scripts/build.sh              # UI shell (needs the Rust mosaic-compile)
bash scripts/bundle-wasm-engine.sh # engine bundle (embeds the Rust engine's .wasm)
```

## Where this fits in the cross-backend demo plan

| Phase | Demo | Status |
|---|---|---|
| 2 | VC2-html (this one) | ✅ live — formulas compute |
| 2 | VC2-webcomp | renders; wiring to engine is the next candidate |
| 2 | VC2-flutter / qt / swiftui / xaml | render (static) |
| 3 | multi-component artifact-builder shells | TODO |
| 4 | code/programs/typescript/visicalc-all/ (all 7 backends side-by-side) | TODO |

See `code/specs/visicalc-cross-backend-demo-plan.md` for the full plan.
The natural next step is teaching the WebComponent demo (`<mos-grid>`)
to talk to the same engine.
