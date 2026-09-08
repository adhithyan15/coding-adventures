# Changelog

## 2026-09-08

- Replace plain startup messages with the generated Mosaic loading/error screen.
  Retry failed loads, dispose abandoned instances, and focus the recovered workbook.

- Query the rendered cell matrix once per presentation-fixture step, preserving
  every assertion while avoiding repeated whole-grid scans on Windows CI.

- Wire generated Open/Save buttons to Mosaic's shared protocol-2 file executor.
  Add real-WASM file-byte round-trip, cancellation, malformed-file and unsupported
  browser tests. Keep status and error presentation in the shared Mosaic root.

## 2026-09-05

- Share the budget seed and versioned presentation fixture under the Mosaic
  sources. Replay 16 edit/navigation steps through generated controls and the
  real Rust/WASM engine; check stored source, computed values and visible slots
  after each step. Production builds now type-check the interaction tests too.
- Keep keyboard selection and inline commits inside the rendered row slice;
  translate generated Grid clicks back to absolute workbook rows and clamp
  selection/scroll bounds. Add three regression cases with the real engine.
  Physical scrolling and responsive viewport sizing remain tracked in #14277.
- Direct formula-bar changes now start an edit session, so Enter writes to the
  spreadsheet engine and Escape restores the prior source. Text-input arrow keys
  no longer navigate the grid.
- Added four generated-control regressions using the real Rust/WASM engine and
  a Linux/Windows CI workflow running both tests and the production build.
- Started the Mosaic reference-application backlog in GitHub issue #14267.
- Commit the dependency lockfile and use `npm ci` in CI so Node type metadata
  and the resolved dependencies agree on clean runners.

## Root Mosaic application

- Replace the React reducer and legacy engine bundle with the Rust MosaicApp lifecycle.
- Generate both root themes and replay existing interaction coverage through real WASM.
