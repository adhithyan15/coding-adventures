# Changelog

## 2026-09-05

- Direct formula-bar changes now start an edit session, so Enter writes to the
  spreadsheet engine and Escape restores the prior source. Text-input arrow keys
  no longer navigate the grid.
- Added four generated-control regressions using the real Rust/WASM engine and
  a Linux/Windows CI workflow running both tests and the production build.
- Started the Mosaic reference-application backlog in GitHub issue #14267.
