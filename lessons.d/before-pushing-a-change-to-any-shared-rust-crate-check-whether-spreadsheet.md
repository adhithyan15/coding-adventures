---
category: CI & GitHub Actions
---

# Before pushing a change to any shared Rust crate, check whether spreadsheet-wasm depends on it; the lexer lesson generalises to parser, grammar-tools and every crate in its wasm cargo tree

**What went wrong.** #18360 bounded the generic `parser` crate's tree size.
Its unit tests passed, but `build (shard-5-of-15)` failed on
`rust/spreadsheet-wasm`: "pkg/spreadsheet_engine.wasm is stale: a source it is
built from changed", and `mosaic/programs/visicalc` was dep-skipped. This is
the same failure the lexer lesson (`a-change-to-the-generic-lexer-crate-...`)
records for #17822, and #18407 hit it too. The trap is that each lesson named
only the one crate involved. The check (`inputs-digest.py`) hashes every path
crate in the wasm target's `cargo tree`, and that tree includes `lexer`,
`parser`, `grammar-tools`, `json-parser` and `xml-parser`, through
`spreadsheet-core` and `spreadsheet-io`.

**The fix.** In `code/packages/rust/spreadsheet-wasm`, with
`CARGO_TARGET_DIR` unset, run `bash build-wasm.sh`. Then run
`bash ../../../programs/typescript/visicalc-html/scripts/bundle-wasm-engine.sh`.
Confirm that both `--check` modes pass and that `node js/smoke.mjs` passes,
then commit `pkg/spreadsheet_engine.wasm`, its `.inputs` stamp and
`visicalc-html/vendor/spreadsheet-engine-wasm.js`.

**What to do differently.** Before pushing a change to a shared Rust crate,
run `cargo tree -p spreadsheet-wasm --target wasm32-unknown-unknown -e
normal,build | grep <crate>`. If the crate is in the tree, rebuild the wasm in
the same PR, after the last source change, because any later edit makes it
stale again.
