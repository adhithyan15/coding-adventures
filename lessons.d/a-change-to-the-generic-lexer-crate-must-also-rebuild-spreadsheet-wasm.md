---
category: CI & GitHub Actions
---

# A change to the generic lexer crate must also rebuild spreadsheet-wasm's committed .wasm and the VisiCalc bundle, because build-wasm.sh --check hashes every crate the wasm depends on

**What went wrong.** #17822 changed only `code/packages/rust/lexer` (F10 mode
transitions on NEWLINE). Its `build (shard-5-of-15)` failed on
`rust/spreadsheet-wasm`: `build-wasm.sh --check` reported
"pkg/spreadsheet_engine.wasm is stale: a source it is built from changed".
The check (`inputs-digest.py`) hashes the `Cargo.toml`, `build.rs` and `src/`
of every path crate in the wasm target's `cargo tree`, and the spreadsheet
formula lexer sits on the generic `lexer` crate. `mosaic/programs/visicalc`
was then dep-skipped. The unit tests all passed, so the failure only shows up
as one package failing in a sharded full build.

**The fix.** Install the target (`rustup target add wasm32-unknown-unknown`),
then run `bash build-wasm.sh` in `code/packages/rust/spreadsheet-wasm` with
`CARGO_TARGET_DIR` unset, because the script reads `$WORKSPACE/target`. Then
run `bash scripts/bundle-wasm-engine.sh` in
`code/programs/typescript/visicalc-html`. Check the result with both
`--check` flags, `node js/smoke.mjs` and `node scripts/verify-infinite.mjs`,
and commit the `.wasm`, its `.inputs` stamp and the vendor bundle.

**Next time.** Before pushing a change to a crate low in the graph (`lexer`,
`parser`, `grammar-tools`, ...), run
`cargo tree -i <crate> -p spreadsheet-wasm --target wasm32-unknown-unknown`.
If it prints anything, regenerate the wasm and bundle in the same PR. Any
other PR that touches those crates will need the same regeneration.
