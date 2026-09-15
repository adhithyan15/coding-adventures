## 0.176.0 — 2026-07-03 — E4d-2b: ALGOL string procedure now runs on Llvm

The ALGOL string-procedure matrix cell (`print(pick(1))` where `pick` returns a
runtime, branch-selected string) gains the **`Llvm`** column, verified
end-to-end via `clang`. This is the payoff of `iir-to-llvm` 0.31.0, which carries
a `str` value as an i64 handle across function boundaries (a `str` parameter /
return / call result) and reads the length header at run time for any string
without a compile-time length — so a *returned* runtime string prints correctly,
not just a branch-selected local. The cell now runs on **NativeAot, Llvm, VM,
JIT**; WASM and JVM/CLR remain (E4d-3b + JVM/CLR analogs of this change). Both
matrix guard tests pass.

