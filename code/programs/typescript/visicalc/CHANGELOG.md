# Changelog

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
