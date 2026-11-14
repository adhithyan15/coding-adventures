---
category: Rust
---

# Cargo test doc mode cannot be combined with lib or named test target selection

A focused Rust validation tried to select the library target, a named
integration-test target, and documentation tests in one `cargo test` command.
Cargo rejects that invocation before compiling because `--doc` is a distinct
test mode and cannot be mixed with the other target selectors.

Run focused executable tests and documentation tests as separate commands:
`cargo test -p <package> --lib --test <target>` followed by
`cargo test -p <package> --doc`. This keeps the scope bounded while ensuring
the validation chain reaches both modes.
