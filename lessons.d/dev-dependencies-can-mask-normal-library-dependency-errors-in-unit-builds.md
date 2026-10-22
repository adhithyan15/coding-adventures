---
category: Rust
---

# Dev dependencies can mask normal library dependency errors in unit builds

A new binder's unit tests passed because `x509-certificate` was available as a
dev dependency, but its normal library build failed during the doc-test command:
a private non-test helper named that crate's type directly. Keep production
helpers within declared normal dependencies (or make test-only helpers
`cfg(test)`), and run a normal `cargo check` or doc build in addition to unit
tests because the unit target includes dev dependencies that can mask this
class of error.
