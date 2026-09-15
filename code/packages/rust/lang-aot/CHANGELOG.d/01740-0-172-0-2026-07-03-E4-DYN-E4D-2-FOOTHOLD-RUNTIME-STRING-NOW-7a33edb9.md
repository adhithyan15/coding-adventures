## 0.172.0 — 2026-07-03 — E4-dyn E4d-2: foothold runtime string now runs on Llvm

The E4-dyn runtime branch-selected-string foothold cell (`INPUT N` picks
`A$ = "HI"`/`"LO"`) gains the **`Llvm`** column, verified end-to-end via real
`clang`. This is the payoff of `iir-to-llvm` 0.30.0, which lowers a runtime
(cross-basic-block) string as an `i64`-handle slot and reads its length from the
block header at run time in `print_str`. The cell now proves the runtime string
on **5 backends** (Llvm, Jvm, Clr, Vm, Jit); `Wasm`/`NativeAot` follow in
E4d-3/E4d-4.

