---
category: Rust
---

# Rust cargo test exact filters require the fully qualified module path

Passing a bare function name together with `cargo test -- --exact` can report
zero tests while still exiting successfully because the harness compares the
filter against the full name, such as `tests::function_name`. Use the qualified
name with `--exact`, or omit `--exact`, and confirm that the result reports the
expected nonzero test count before treating a focused run as evidence.
