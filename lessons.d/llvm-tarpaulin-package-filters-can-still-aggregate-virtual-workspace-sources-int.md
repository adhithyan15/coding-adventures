---
category: Testing & coverage
---

# LLVM tarpaulin package filters can still aggregate virtual-workspace sources into the threshold

Running `cargo tarpaulin -p rsa-public-operation --engine llvm --all-targets
--fail-under 95` from the Rust virtual workspace executed only the selected
crate's tests, yet the report mapped source files from the entire workspace.
The new library measured 62/62 lines, but the command failed because untouched
workspace sources reduced the aggregate to 0.80 percent. Treat the per-file
package row as local evidence on this configuration and leave the blocking
threshold to the repository's Linux package-scoped coverage runner; do not
weaken tests or add unrelated exclusions to make a virtual-workspace aggregate
look like package coverage.
