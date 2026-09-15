## 0.173.0 — 2026-07-03 — E4-dyn E4d-3: foothold runtime string now runs on Wasm

The E4-dyn runtime branch-selected-string foothold cell (`INPUT N` picks
`A$ = "HI"`/`"LO"`) gains the **`Wasm`** column, verified end-to-end in-process
via the `wasm-runtime` interpreter and its `env.__print_str` host. This is the
payoff of `iir-to-wasm` 0.29.0, which promotes a runtime (cross-basic-block)
string to an i32 **handle** = the offset of a length-prefixed block
`[i32 len][bytes]` in linear memory, and reads the length back with `i32.load`
in `print_str`. The cell now proves the runtime string on **6 backends**
(Llvm, Wasm, Jvm, Clr, Vm, Jit); only `NativeAot` remains, added by E4d-4.

