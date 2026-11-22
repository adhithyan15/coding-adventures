---
category: Rust
---

# Cargo fmt may rewrite untouched baseline files in a package

Running `cargo fmt -p game-clock` reformatted three pre-existing source/test files even though this
change only added a new integration test. Those formatting-only differences were unrelated to the
feature and were restored before proceeding. For small test additions, format the new file directly
with `rustfmt` or run the formatter and inspect/revert every unrelated package diff before staging.
