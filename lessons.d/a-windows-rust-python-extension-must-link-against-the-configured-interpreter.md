---
category: Native extensions & FFI
---

# A Windows Rust Python extension must link against the configured interpreter

Running `cargo test` directly for `python/heap-native` resolved the shell's
default Python 3.10, while the package requires Python 3.12 or newer. Its build
script then failed to find the matching import library, and the test link
reported unresolved `Py*` symbols. Set `PYO3_PYTHON` to the absolute path of
the same compatible interpreter used by the package build, and use a fresh
task-local Cargo target directory so the build script runs with that setting.
The earlier link error was an invocation environment mismatch, not a heap
crate failure.
