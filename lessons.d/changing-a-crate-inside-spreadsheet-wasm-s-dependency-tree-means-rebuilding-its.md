---
category: CI & GitHub Actions
---

# Changing a crate inside spreadsheet-wasm's dependency tree means rebuilding its committed wasm and the VisiCalc bundle

`spreadsheet-wasm/pkg/spreadsheet_engine.wasm` is a committed build artifact.
Since #17810, CI runs `build-wasm.sh --check`. That check hashes the source of
every crate in `cargo tree -p spreadsheet-wasm --target wasm32-unknown-unknown`
and compares the hash with the `.inputs` stamp committed next to the `.wasm`.

PR #17803 changed only `lexer`, but `lexer` and `grammar-tools` sit in that
tree. Shard 5 of the forced full build therefore failed with "pkg/
spreadsheet_engine.wasm is stale: a source it is built from changed", even
though nothing spreadsheet-related was touched.

The check came from main after the PR was cut. It only showed up because CI
builds the PR merged with main, so the branch's own tree had no such check.

Fix:
1. Merge main into the branch.
2. Run `bash build-wasm.sh` in `code/packages/rust/spreadsheet-wasm`, then
   `node js/smoke.mjs`.
3. Run `bash code/programs/typescript/visicalc-html/scripts/bundle-wasm-engine.sh`.
4. Confirm with `cargo test --test web_demos_are_current` in
   `code/programs/mosaic/visicalc`.
5. Commit the `.wasm`, the `.inputs` stamp and `vendor/spreadsheet-engine-wasm.js`
   together.

Before pushing a change to any shared Rust crate, run
`cargo tree -i <crate> -p spreadsheet-wasm --target wasm32-unknown-unknown`.
If it prints anything, the artifact needs rebuilding.
