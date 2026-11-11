---
category: Testing & coverage
---

# cargo llvm-cov fails with Argument list too long in the monorepo workspace; run instrument-coverage by hand with rustup's llvm-profdata

`cargo llvm-cov -p <crate>` in `code/packages/rust` failed with `could not execute
process cargo test ... (never executed): Argument list too long (os error 7)`.
cargo-llvm-cov puts every workspace crate name into one environment variable
(`__CARGO_LLVM_COV_RUSTC_WRAPPER_CRATE_NAMES`), which is about 144 KB here and
exceeds the kernel's 128 KiB limit for a single environment string.

Run the instrumentation by hand instead: set `RUSTFLAGS="-C instrument-coverage"`,
a separate `CARGO_TARGET_DIR` and `LLVM_PROFILE_FILE=<dir>/%p-%m.profraw`, run
`cargo test -p <crate>`, merge with the **rustup toolchain's**
`lib/rustlib/<triple>/bin/llvm-profdata` (the distribution's `/usr/bin` copy reads
an older raw-profile version and says "no profile can be merged"), then
`llvm-cov export -format=lcov -object <each test binary>` and intersect the lcov
with `git diff -U0` to get the coverage of the lines a branch changed.  Delete the
instrumented target directory afterwards: it is several GB.
