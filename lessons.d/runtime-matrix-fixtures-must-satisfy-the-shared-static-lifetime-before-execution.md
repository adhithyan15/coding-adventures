---
category: Rust
---

# Runtime matrix fixtures must satisfy the shared static lifetime before execution

The current-main native880 validator reached Cargo test101 before any test
harness executed: lang-aot's new generated Vec<String> fixtures could not satisfy
Prog.src/Expect::Stdout's static lifetime (E0597). The later validator identity
mismatch was a consequence of partial compilation, not the primary failure.
Core926 and compiler1139 successes do not establish these runtime cells.

Read the existing native-matrix-programs-require-static-source-fixtures lesson.
Materialize bounded source/expected-output tables as immutable literal fixtures,
prove all24 strings and ordering match the prior generator, preserve all normal
Exit0/stdout/selected-order checks, and do not leak allocations or weaken the
shared Prog contract. Preserve the failed log/receipt and rerun to terminal exit.
