# Changelog — visicalc-webcomp

## Unreleased

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
