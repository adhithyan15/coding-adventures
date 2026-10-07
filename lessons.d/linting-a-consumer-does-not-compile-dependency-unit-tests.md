---
category: Rust
---

# Linting a consumer does not compile dependency unit tests

The CLOC31 local all-target Clippy runs covered `closurec`, `javascript-ast`
and `closure-pass-constant-fold`. They passed, but macOS affected-package CI
failed while linting `closure-pass-pipeline`'s own test target: `CountingPass`
used an atomic method deprecated by CI's newer Rust toolchain. The consumer
compiled the dependency as a library, so its private test helper was absent.

When a shared crate changes, validate relevant downstream package test targets
as well as its consumers. `cargo clippy --all-targets` means all targets of
the selected packages, not every dependency's unit tests. Classify CI failures
from the actual failing step and log; this was a strict test-target lint failure,
not a bootstrap or compiler-runtime failure. Preserve test behavior with a
compatible API rather than suppressing deprecation warnings.
