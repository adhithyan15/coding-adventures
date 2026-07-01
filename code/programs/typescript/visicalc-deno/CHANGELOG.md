# Changelog

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
