## 0.582.0 (UNRELEASED)

- Two seven-backend ALGOL matrix rows pin that `entier` of a runtime real no
  integer can hold fails closed. One is finite (`entier(exp(exp(pick())))`,
  about 5.1e23); the other overflows to +∞. Before this release, native x86-64
  printed a sign-flipped `i64::MIN`, and the JVM and aarch64 saturated to
  `i64::MAX`. `x86_64-backend` 0.41.0, `iir-to-jvm-class-file` 0.37.0 and
  `aarch64-backend` now trap like the VM, LLVM, WASM and CLR backends.
