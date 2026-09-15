## 0.211.0 - 2026-07-13 — E6d-6: Twig unions / match on the code-gen backends

Two matrix cells prove Twig **unions + `match`** dispatch on all five code-gen
backends:

- `(union Opt (Some (v : int)) (None)) (match (Some 42) ((Some v) v) ((None) 0))` → 42
- `(union Opt (Some (v : int)) (None)) (match (None) ((Some v) v) ((None) 42))` → 42

on `[NativeAot, Llvm, Wasm, Jvm, Clr]`. A union erases (frontend, unchanged) to
integer-tagged constructors — a cons `(tag . fields…)` — and `match` compares the
scrutinee's tag (`car`) to each variant's integer tag, binding fields via
`car(cdr^i)`, over the E6d-1 heap substrate. The second cell (matching the *second*
variant) proves the tag test actually discriminates rather than always taking the
first arm.

Two fixes made it run on the managed + native backends:

1. **twig-ir-compiler 0.44.0** — the variant tag constant is typed `i64`, not
   `"any"` (an `"any"` const was wrongly `unbox`ed by `lower_dynamic_arith` → WASM
   `expected i32, got I64` trap).
2. **iir-builtin-lowering 0.29.0** — a boxed-bool `jmp_if_false` (the boxed `=`
   tag-compare result) now branches on its RAW bool. Both dyn_repr passes
   previously wrapped it with nil-/McCarthy-truthiness, but a boxed `#f` is a
   non-nil i31 / tagged-0, read as true → every arm mis-matched. Fixed in both the
   structural pass (WASM/JVM/CLR/BEAM) and the native `dyn_truthy` path
   (NativeAot/LLVM).

Run-verified locally on WASM (in-process) + real dotnet CLR — including that the
full matrix (every prior merged cell) still agrees, since fix #2 touches every
managed conditional; native/LLVM/JVM via CI.

