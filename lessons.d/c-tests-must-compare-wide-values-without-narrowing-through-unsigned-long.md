---
category: Cross-platform & Windows BUILD_windows
---

# C tests must compare wide values without narrowing through unsigned long

The C HDD tests passed under GCC and Clang on Linux. MSVC rejected the
clock-ceiling assertions because the shared unsigned comparison helper
converts both operands to `unsigned long`. On Windows x64 that type is
32 bits wide. The simulator intentionally uses 64-bit ticks, and the
53-bit clock ceiling crosses that boundary. A narrowed comparison can
also hide a discrepancy in the upper bits of two runtime values.

Compare wide values with a full-width C predicate or a fixed-width
assertion. Keep intentional masks in the destination range and check
for shadowed locals under the normal warnings-as-errors build. A
native Windows build checks the ABI and diagnostic behavior exercised
by the repository's Windows command list.

[Windows build for the C HDD simulator](https://github.com/adhithyan15/coding-adventures/actions/runs/46007754878/job/140535966774)
