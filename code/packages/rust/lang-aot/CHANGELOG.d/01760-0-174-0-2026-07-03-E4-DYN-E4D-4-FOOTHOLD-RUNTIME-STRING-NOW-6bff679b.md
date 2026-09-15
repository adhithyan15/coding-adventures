## 0.174.0 — 2026-07-03 — E4-dyn E4d-4: foothold runtime string now runs on NativeAot (all 7 backends)

The E4-dyn runtime branch-selected-string foothold cell (`INPUT N` picks
`A$ = "HI"`/`"LO"`) gains the **`NativeAot`** column — completing the E4-dyn
backend ladder. The runtime string now runs on **all seven backends**
(NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit), all agreeing on `"HI"`. aarch64 is
run-verified locally on this Apple Silicon host; x86_64 is verified on the Linux
CI runner through the same matrix cell.

This is the payoff of `twig-aot` 0.27.0: on native, `str_const` already builds a
`[i64 len][bytes]` heap buffer and stores its address in the variable's stack
slot, and `print_str` already has a runtime length-read path (`field_load` the
header). The E4d-4 fix simply keeps a branch-selected string out of `twig-aot`'s
compile-time literal map so `print_str` takes that runtime path instead of folding
one branch's static length — one change covering both the aarch64 and x86_64
backends. `matrix_every_proven_cell_agrees` + `proven_columns_do_not_silently_skip`
pass with `NativeAot` added.

