---
package: c/iso-harness
lane: c
rung: E1
status: open
---

# iso_test.h treats a NaN as within tolerance of any value

`ISO_CHECK_EQ_DBL` in `code/packages/c/iso-harness/include/iso_test.h`
fails when `difference > eps`. Every comparison with NaN is false, so a NaN
result **passes** the check against any expected value. About 289 C and C++
BUILDs use the harness, so any of their floating-point checks could be
vacuous for NaN.

**Open:** fail unless `difference <= eps` (which is false for NaN). This is
also the first entry for the test-framework corpus's mandatory negative cases
(TF00).
