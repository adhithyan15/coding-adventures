## 0.196.0 — 2026-07-11 — LANG-FULL E6d-2a: dynamic integer arithmetic over `any` (structural backends)

Wires the new `iir_builtin_lowering::lower_dynamic_arith` pass into every managed + native lisp lowering pipeline (after `lower_heap_builtins`), and adds a matrix cell `(+ (car (cons 41 0)) 1)` → 42 on the **structural** code-gen backends `[Wasm, Jvm, Clr]` (whose `i31ref`/`Integer`/boxed-int32 box/unbox now width-adapt to i64). NativeAot + LLVM (NaN-box tagged-i64 world) follow in E6d-2b. Run-verified: WASM in-process, JVM via a reflective launcher (both → 42); CLR via CI.

