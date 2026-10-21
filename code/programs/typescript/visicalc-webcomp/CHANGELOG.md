# Changelog — visicalc-webcomp

## Unreleased

### Fixed — vendor elements regenerated for #17807

- #17807 taught the Web Component emitter to keep focus across re-renders and
  to dispatch `onChange` per keystroke (`oninput` plus `oncompositionend`), but
  `vendor/FormulaBar.js` and `vendor/Grid.js` were not regenerated, so
  `scripts/build.sh --check` (the `web_demos_are_current` test) failed on main.
  Both are regenerated with `bash scripts/build.sh`.

### Changed — generated pieces are written by scripts and checked

- `scripts/build.sh` writes `vendor/FormulaBar.js` and `vendor/Grid.js` in place
  (they used to be compiled into `build/` and copied by hand), and `--check`
  fails if they differ from what `mosaic-compile` produces today. Both were
  stale and are regenerated.
- The page loads the HTML demo's engine bundle
  (`../visicalc-html/vendor/spreadsheet-engine-wasm.js`). This demo's own copy,
  and its fork of the loader, are removed: the copy embedded an engine that
  matched no committed `.wasm`, and the loader lacked the file, format, sheet,
  undo and viewport APIs the HTML one has.
- The check runs in CI from `code/programs/mosaic/visicalc`'s
  `web_demos_are_current` tests.
