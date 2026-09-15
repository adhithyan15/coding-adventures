# clang `@llvm.floor.f64`/math intrinsics need `-lm` on Linux (E8 PR-2, #6584)

A RUN-verified iir-to-llvm test that compiled a program using `@llvm.floor.f64`
passed locally on macOS but failed `build (ubuntu-latest)` with `clang: error:
linker command failed`. Cause: at `-O0` (the default for `clang -x ir`) the
`@llvm.floor.f64`/`@llvm.trunc.f64` intrinsics lower to libm `floor`/`trunc`
*calls*, which on Linux must be linked with `-lm`. macOS libm lives in libSystem
(linked by default), so the gap is invisible locally. Fix: add `.arg("-lm")` to
the clang invocation. LESSON: any LLVM program using a floating-point math
intrinsic (floor/trunc/sin/sqrt/…) must link `-lm`; test it on Linux CI, and the
real entier matrix pipeline (E8 PR-7) must link `-lm` too.
