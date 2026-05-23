# VisiCalc — WebComponent demo

Second cross-backend visual demo (Phase 2 / VC2-webcomp), driven by a
real Web Component. Companion to `demo/visicalc-html/`'s static
snapshot.

## What it shows

- `<mos-formula-bar cell-address="A1" formula="=SUM(B1:B5)">` rendered
  as a live custom element with its own shadow DOM. Styles compiled
  in from `FormulaBar.dark.msl`.
- Typing into the input dispatches `mosaic:formulaChange` /
  `mosaic:commit` / `mosaic:cancel` CustomEvents that escape the
  shadow root (because the emitter sets `bubbles: true, composed:
  true`).
- A small event log at the bottom shows the events as they fire —
  proof the wiring works end-to-end.
- A 5×5 sample grid below (hand-written for now — see the gap note).

## How it's built

```bash
bash scripts/build.sh
```

Runs `mosaic-compile --backend webcomponent` against
`demo/visicalc/mosaic/FormulaBar.{mil,desktop.mll,dark.msl}` and
writes `build/FormulaBar.js`. The bundle is a self-registering
script: it calls `customElements.define("mos-formula-bar", ...)` at
parse time.

## The Grid gap

The `Grid` built-in primitive isn't yet supported by the
`mosaic-emit-webcomponent` pipeline (only the React emitter knows how
to lower it — see
`code/packages/rust/mosaic-emit-react/src/pipeline.rs`). Until the
WebComponent Grid emitter lands, the grid below the formula bar in
`index.html` is hand-written to mirror what the eventual `<mos-grid>`
custom element should render.

## How to view

```
open demo/visicalc-webcomp/index.html
```

(Or just double-click the file in a file browser. Type into the
formula bar and watch the events stream into the log below.)

## Where this fits in the cross-backend demo plan

| Phase | Demo | Status |
|---|---|---|
| 2 | VC2-html | ✅ |
| 2 | VC2-webcomp (this one) | ✅ |
| 2 | VC2-flutter | TODO |
| 2 | VC2-qt | TODO |
| 2 | VC2-swiftui | TODO |
| 2 | VC2-xaml | TODO |
| 3 | multi-component artifact-builder shells | TODO |
| 4 | demo/visicalc-all/ (all 7 backends) | TODO |
