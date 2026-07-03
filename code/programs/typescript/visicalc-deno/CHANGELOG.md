# Changelog

## 0.2.0

- Add the **FFI native-engine variant** (`main-ffi.ts`): loads
  `libspreadsheet_capi` (the shared Rust `spreadsheet-core` C ABI) into the Deno
  process via `Deno.dlopen` and serves a small HTTP API (`/api/window`,
  `/api/raw`, `/api/cell`) that a thin webview client renders + posts edits to.
  Where `main.ts` runs the engine as WebAssembly in the browser, this runs it as
  **native machine code server-side** — the same engine, two transports.
- `scripts/build-engine.sh` (`deno task engine`) builds the crate to a C ABI
  dynamic library and vendors it into `vendor/` (git-ignored build artifact).
- New tasks: `engine`, `dev:ffi`, `desktop:ffi`, `build:ffi`.
- Verified on macOS: the API serves the computed cross-footing budget
  (E1=38 … E5=169); editing A1 15→100 recomputes E1→123, A5→124, E5→254 through
  the native library (confirmed via curl and the webview client in a browser).

## 0.1.0

- Initial VisiCalc host on **Deno Desktop** (Deno 2.9 `deno desktop`) — the tenth
  cross-backend host. Serves the engine-backed web VisiCalc (the HTML demo's
  `index.html` + the WASM `spreadsheet-core` engine) via `Deno.serve()`; `deno
  desktop` wraps it in a native webview window.
- Assets (`index.html` + `vendor/spreadsheet-engine-wasm.js`) are embedded via
  text-import attributes so the compiled `.app` is self-contained (no runtime
  filesystem dependency on the sibling demo).
- Verified on macOS: the window renders computed values (E-column row sums, row-5
  column sums, grand total 169); editing a cell recomputes live.
