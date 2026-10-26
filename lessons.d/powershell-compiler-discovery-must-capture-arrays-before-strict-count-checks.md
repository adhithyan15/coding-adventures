---
category: Cross-platform & Windows BUILD_windows
---

# PowerShell compiler discovery must capture arrays before strict Count checks

NN47 exposed a shared ISO-harness StrictMode failure with exactly one compiler:
PowerShell unrolled discovery into a scalar, so `$compilers.Count` was not a
valid property on Windows PowerShell 5. Both positive and negative compile
guards must use `@(Iso-Compilers ...)` before checking cardinality.

Six regressions invoke the production guards for zero, one and two compiler
results. A sentinel at the later build-directory boundary proves the Count
guard was actually traversed without needing fake executables or relaxing
compiler requirements. The real MSVC package build still supplies compilation
and runtime evidence separately.
