# Changelog — visicalc-html

## Unreleased

### Changed — engine bundle regenerated

- `vendor/spreadsheet-engine-wasm.js` re-embeds `spreadsheet-wasm`'s rebuilt
  `.wasm` after the generic lexer changed (#17824).

  `.wasm` after the generic lexer changed (#17822).

### Changed — generated pieces are written by scripts and checked

- `scripts/build.sh` writes the compiled FormulaBar and Grid fragments into
  `index.html`'s `<template>`s itself (`scripts/sync-templates.py`) instead of
  leaving them to be pasted. `--check` compiles into a temp directory and fails
  if the page differs. The page had drifted: it lacked the per-row heights and
  the cell editor's accessible name, autofocus and geometry (#15048), which it
  now has.
- `scripts/bundle-wasm-engine.sh --check` fails if `vendor/spreadsheet-engine-wasm.js`
  is not what the committed `.wasm` and `scripts/wasm-loader.js` produce. The
  bundle embedded neither the committed `.wasm` nor a current engine; it is now
  rebuilt from a `.wasm` freshly built from today's spreadsheet-core, so fixes
  such as #15541 reach the demo. `verify-infinite.mjs` passes against it.
- Both checks, and `verify-infinite.mjs`, run in CI from
  `code/programs/mosaic/visicalc`'s `web_demos_are_current` tests.
- `touch.html` is removed: the theme and layout switcher in `index.html`
  supersedes it, and its grid was a hand-written placeholder with wrong seed
  data.
