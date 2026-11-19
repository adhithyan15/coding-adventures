---
category: Rust
---

# The Rust build plan omits its strict all-target Clippy preflight

On PR #19685, the downloaded build-plan artifact listed the Rust crate's
`BUILD` commands (`cargo test` and `cargo test --release`) but did not display
the build tool's implicit strict Clippy preflight. The first macOS package job
ran Rust 1.100.0 `cargo clippy --all-targets -- -D warnings` before those
commands and rejected both a nested `if` in the new library and a
range-indexed loop in its integration-test oracle. Debug and release tests had
passed locally, so inspecting only the package command list missed the actual
CI gate.

Both lint findings were fixed without changing the reducer/proof semantics or
the reference oracle's 64-bit ordering. Re-running strict Clippy locally with
the CI toolchain caught and closed both findings before the replacement CI run;
Ubuntu, macOS and Windows then reported the bridge package `BUILT`.

**Do instead:** when reading or reproducing build-tool output, inspect the
effective executor recipe and preflights, not only the downloaded plan's
declared `build_commands`. For every touched Rust crate, run
`cargo clippy -p <crate> --all-targets -- -D warnings` on the toolchain used by
CI before treating tests as a passing pre-push check. Record the preflight and
its flags in future per-package receipts; see GBL-059.
