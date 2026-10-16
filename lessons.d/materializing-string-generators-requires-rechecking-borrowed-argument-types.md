---
category: Rust
---

# Materializing string generators requires rechecking borrowed argument types

A generated String loop became an explicit array of complete &str programs. The
old compile_source(&source, ...) assertion still executed correctly, but strict
Clippy at bb838 rejected the now-redundant borrow. All1002 package tests passed;
strict exited101, so the combined receipt remains unaccepted. Removing that one
borrow preserves the exact diagnostic/value assertion. When materializing a
generator, include binding types and all-target strict lint in the migration;
record this mechanical difference in the whole-function inverse rather than
silently rewriting historical oracle evidence.
