---
category: Repo policy / workflow reminders
---

# Do not run tarpaulin beside another Cargo command in the same target directory

`cargo tarpaulin` cleans and rebuilds the Cargo target directory as part of its
instrumentation workflow. Running it in parallel with `cargo clippy`, `cargo
test`, or `cargo doc` for the same workspace can delete dependency files while
the other Cargo process is writing them, producing misleading `No such file or
directory` compiler failures even after both commands waited on Cargo locks.

Run coverage sequentially with other Cargo commands, or assign it an isolated
`CARGO_TARGET_DIR`. Cargo's package-cache and artifact locks do not make a
tarpaulin clean safe beside another build.
