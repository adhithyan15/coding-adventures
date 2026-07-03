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

## FFI variant (`main-ffi.ts`) — native engine via `Deno.dlopen`

A second host runs the **same** engine, but as **native machine code** instead of
WebAssembly. `main-ffi.ts` loads `libspreadsheet_capi` (the C ABI the Qt and
SwiftUI demos link) into the Deno process with `Deno.dlopen`, and serves a tiny
HTTP API that the webview calls:

```
main-ffi.ts  ──Deno.dlopen──▶  libspreadsheet_capi.{dylib,so,dll}   (native Rust engine)
     │  Deno.serve  (/api/window, /api/raw, /api/cell)
     ▼
  webview (thin client): fetches computed values, posts edits
```

Where `main.ts` runs the engine **in the browser** (WASM), `main-ffi.ts` runs it
**server-side in Deno** (native FFI) and the webview is pure I/O — the same
engine, two transports, mirroring the native-engine path of the Qt/Flutter demos.

```
deno task engine        # build + vendor libspreadsheet_capi (prerequisite)
deno task desktop:ffi   # open a dev window (deno desktop -A main-ffi.ts)
deno task dev:ffi       # server only — open http://localhost:8792
deno task build:ffi     # compile a self-contained VisiCalc-FFI.app
```

The vendored `vendor/libspreadsheet_capi.*` is a build artifact (git-ignored,
rebuilt by `deno task engine`). The FFI host needs `--allow-ffi --allow-read`
(the `dev:ffi` task sets them; `deno desktop` runs with `-A`). Verified: the
API serves the computed budget (E1=38 … E5=169) and an edit (A1 15→100)
recomputes E1→123, A5→124, E5→254 — through the native library, not WASM.
