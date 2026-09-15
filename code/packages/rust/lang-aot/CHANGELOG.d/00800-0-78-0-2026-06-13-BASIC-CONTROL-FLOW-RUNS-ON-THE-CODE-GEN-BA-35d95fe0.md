## 0.78.0 — 2026-06-13 — BASIC control flow runs on the code-gen backends (LANG-FULL BA0)

`tests/lang_matrix.rs` gains two executed Dartmouth BASIC programs exercising real
control flow — a `FOR`/`NEXT` accumulator (`FOR I = 1 TO 5: S = S + I` → prints
**15**) and an `IF A > 5 THEN 100` jump (→ prints **7**) — across native / LLVM /
WASM / CLR / VM / JIT. Until now BASIC loops/conditionals executed only on the
VM/JIT. Backed by `dartmouth-basic-iir-compiler` 0.5.0, which fixes the comparison
`type_hint` (`bool` → `i64` operand width) that had LLVM comparing at a 1-bit `i1`
width. JVM is excluded for these two programs pending BA-JVM-1 (a StackMapTable
follow-up for branch + `print_i64`). No `lang-aot/src` change.

