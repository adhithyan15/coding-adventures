---
category: Testing & coverage
---

# Cargo llvm-cov can exceed Windows command-line limits in a large workspace

For VER26's CT01 adapter, the instrumented tests all passed, but
`cargo llvm-cov test -p coding_adventures_ct_compare --summary-only` could not
launch its final reporter: Windows error 206 (command line too long). The tool
expanded an exclusion for every unselected package in the large Rust workspace.
This was a report-launch failure, not a failing test or a coverage result.

Use the installed toolchain's llvm-cov directly with the same rust.profdata and
exact instrumented test executables, plus a short filename exclusion for test
and dev-dependency code. That produced the real report (145/145 ct-compare source
lines covered) without changing source, tests, workspace membership or OS limits.
Keep all object files needed for the selected package; never infer coverage from
the fact that the instrumented tests ran. A general cargo-llvm-cov Windows
response-file repair belongs to its own tooling work item if needed again.
