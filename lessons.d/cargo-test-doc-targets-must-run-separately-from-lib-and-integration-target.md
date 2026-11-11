---
category: Rust
---

# Cargo test doc targets must run separately from lib and integration target selectors

One validation command combined Cargo's `--doc` selector with `--lib` and an
explicit `--test` target. Cargo rejects that combination before compilation, so
the command provided no test result even though each intended target was valid.

Run library and integration targets together when useful, then run documentation
tests in a separate `cargo test -p <package> --doc` command. Treat a selector
usage error as an unexecuted check, never as a passing or failing code result.
