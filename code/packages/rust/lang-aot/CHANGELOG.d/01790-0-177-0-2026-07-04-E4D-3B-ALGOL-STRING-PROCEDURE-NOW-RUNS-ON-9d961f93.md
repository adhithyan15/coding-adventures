## 0.177.0 — 2026-07-04 — E4d-3b: ALGOL string procedure now runs on Wasm

The ALGOL string-procedure matrix cell (`print(pick(1))` where `pick` returns a
runtime, branch-selected string) gains the **`Wasm`** column, verified
end-to-end in-process via the `wasm-runtime` interpreter. This is the payoff of
`iir-to-wasm` 0.30.0, which reads the string length from the `[i32 len][bytes]`
block header at run time for any string lacking a compile-time literal entry (a
call result / return value / parameter), not only a promoted slot — so a
*returned* runtime string prints correctly. The cell now runs on **NativeAot,
Llvm, Wasm, VM, JIT**; only JVM/CLR remain (the same runtime-return change on
those backends). Both matrix guard tests pass.

