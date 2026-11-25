---
category: Native extensions & FFI
---

# POSIX C math calls need libm after object inputs

Some ISO C math functions are out-of-line symbols on POSIX even when the
compiler accepts their declarations. A test that calls `nextafter` linked on
Windows but failed under both Linux GCC and Clang with an undefined reference.
For those builds, pass `-lm` after the test and library source/object inputs;
traditional linkers resolve library symbols left to right. Keep this test-only
link option off installed targets when the library implementation itself does
not call an out-of-line math function. Verify the real Linux linkers in CI, not
only MSVC or a local Windows GNU toolchain.
