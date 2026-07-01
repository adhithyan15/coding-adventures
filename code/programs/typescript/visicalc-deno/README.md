# VisiCalc — Deno Desktop demo

Tenth cross-backend VisiCalc host. Uses **Deno 2.9's `deno desktop`**, which
bundles your code + the Deno runtime + a webview into one native app per
platform and opens a window pointed at your `Deno.serve()` handler.

Like the Electron host (which wraps the React bundle), this "wrapper" reuses the
web VisiCalc — but here the webview is Deno's own, with **zero third-party
dependencies**. It serves the exact same engine-backed page the HTML demo runs:
the shared Rust `spreadsheet-core` engine compiled to WebAssembly. The grid shows
the engine's *computed* values (E-column row sums, row-5 column sums, grand total
169) and edits recompute live.

## How it fits

```
code/programs/mosaic/visicalc/{FormulaBar,Grid}.*   (Mosaic UI source)
        │  mosaic-compile --backend html
        ▼
code/programs/typescript/visicalc-html/             (engine-backed web app)
   index.html + vendor/spreadsheet-engine-wasm.js  ── embedded via text imports ──┐
                                                                                   ▼
                                                        main.ts  ── deno desktop ──▶  native window
```

`index.html` and the one script it loads (`vendor/spreadsheet-engine-wasm.js`,
the base64-embedded WASM engine) are embedded into `main.ts` via text-import
attributes, so the compiled `.app` is self-contained.

## Run

```
deno task desktop   # open a dev window  (deno desktop -A main.ts)
deno task build     # compile a self-contained VisiCalc.app
deno task dev       # server only — open http://localhost:8791
```

## Follow-up

A native-engine variant that loads `libspreadsheet_capi` through `Deno.dlopen`
(FFI) instead of WASM — mirroring the Qt/Flutter/SwiftUI native-engine path — is
tracked separately.
