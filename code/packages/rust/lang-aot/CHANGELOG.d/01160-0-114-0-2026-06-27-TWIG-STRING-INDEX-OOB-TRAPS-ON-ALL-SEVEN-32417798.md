## 0.114.0 — 2026-06-27 — Twig string index OOB traps on all seven backends (LANG-FULL E4)

The Twig matrix now treats runtime traps as a first-class expected result via
`Expect::Trap`, and executes `(string-ref "ABC" 3)` on **native-AOT + LLVM +
WASM + JVM + CLR + VM + JIT**. Every backend fails closed instead of returning a
byte or silently skipping the cell.

This closes the E4 `str_index` out-of-bounds proof for the current literal/named
string foothold: WASM, VM, and JIT use their existing bounds checks; JVM/CLR use
their managed string index exceptions; and native AOT plus LLVM now lower a
compile-known literal OOB index to a runtime trap path instead of rejecting during
lowering.

