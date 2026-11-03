---
package: c/iso-harness
lane: c
rung: E1
status: mitigated
---

# iso_test.h treats a NaN as within tolerance of any value

`ISO_CHECK_EQ_DBL` in `code/packages/c/iso-harness/include/iso_test.h`
fails when `difference > eps`. Every comparison with NaN is false, so a NaN
result **passes** the check against any expected value. About 289 C and C++
BUILDs use the harness, so any of their floating-point checks could be
vacuous for NaN.

**Handling (mitigated):** the check now passes only when `a == b`, both
values are NaN, or `|a - b| <= eps`, which is false for NaN. Equal infinities
still pass through `a == b`. `selftest/falsifiable.c` and `.cpp` prove every
iso_test.h check can fail, on every compiler, and fail against the old
header. It is the C lane's first mandatory-negative fixture, as TF01 asks of
every harness.
