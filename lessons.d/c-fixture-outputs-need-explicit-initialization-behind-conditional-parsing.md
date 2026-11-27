---
category: Cross-platform & Windows BUILD_windows
---

# C fixture outputs need explicit initialization behind conditional parsing

The C HDD fixture runner fills an expected timing aggregate through
a parser on successful cases. Overflow cases use a separate path.
MSVC still issued C4701 at a later read under the same success guard.
The Windows harness treated that warning as an error.

Initialize the aggregate at declaration and keep the parser's failure
check on the success path. Retain the exact fixture and error-case
assertions. Test with optimized GCC and Clang uninitialized-value
diagnostics in addition to native MSVC.

[Windows build showing C4701](https://github.com/adhithyan15/coding-adventures/actions/runs/46018127593/job/140571206579)
