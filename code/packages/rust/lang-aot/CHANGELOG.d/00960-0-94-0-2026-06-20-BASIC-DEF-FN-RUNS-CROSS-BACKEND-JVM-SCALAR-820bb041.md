## 0.94.0 — 2026-06-20 — BASIC `DEF FN` runs cross-backend; JVM scalar concretization is module-consistent (LANG-FULL BA5)

### Added — executed matrix proof for BASIC user-defined functions

`tests/lang_matrix.rs` adds `DEF FNS(X) = X * X : PRINT FNS(7)` → stdout `49`
on **all 7 backends** (native/LLVM/WASM/JVM/CLR/VM/JIT). This RUNS a real
cross-function `call` combined with `print_i64`, proving a same-module function
call resolves and executes on the code-gen backends, not just the VM/JIT.
(`dartmouth-basic-iir-compiler` 0.6.0 lowers `DEF FN`.)

### Fixed — `concretize_scalar_any_for_jvm` decided per-function, breaking cross-function calls

The JVM scalar-concretization pass narrows a scalar function's `i64` values to
`i32` (the in-repo `jvm-simulator` is 32-bit) **unless** the function prints —
a printing function keeps the wide i64 model because `println(J)V` needs a
`long`. But this was a **per-function** decision, and a `call` couples two
functions' value models: the caller pushes the argument and consumes the result
at the callee's declared width. The BA5 program lowers to a printing `main`
(kept i64) plus a non-printing helper `FNS` (narrowed to `(I)I`), so `main`
`invokestatic`'d `FNS` with a `long` argument and `lstore`'d an `int` result —
real `java` rejected the mismatch with a `VerifyError` and the program printed
nothing.

Concretization is now a **whole-module** decision: if **any** function in the
module prints, every scalar function stays at i64, keeping all cross-function
call signatures consistent. A module with no printing function (Nib/Twig/ALGOL,
which return an exit code) still concretizes to i32 uniformly — unchanged. Only
printing multi-function modules are affected.

