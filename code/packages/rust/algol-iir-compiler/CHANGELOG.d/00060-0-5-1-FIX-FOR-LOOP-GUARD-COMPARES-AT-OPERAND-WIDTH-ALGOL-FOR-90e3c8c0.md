## 0.5.1 — Fix: `for`-loop guard compares at operand width (ALGOL `for` runs on LLVM)

The `for … step … until` loop-guard comparisons (the step-sign check and the
ascending/descending bound checks) were emitted with `type_hint = "bool"` — the
boolean *result* type — instead of the integer *operand* width. A code-gen
backend reads a comparison's `type_hint` as its operand type, so on **LLVM**
(`iir-to-llvm`'s `lower_cmp`) this produced the invalid `icmp i1 <i64>, <i64>`
that `clang` rejects, leaving ALGOL `for` loops un-runnable on LLVM (they worked
on VM/JIT/JVM/CLR, which infer operand types differently). The three guards now
carry `"i64"`, matching the regular relational path (which already tags the cmp
with `lhs.ty.iir()`).

Effect: ALGOL `for`-loop programs now compile to valid LLVM IR and **run via
`clang`**. The E5 sum-of-squares array `Prog` (two `for` loops over an array,
exit 55) — previously LLVM-deferred — now runs on the LLVM matrix column. New
regression test asserts the guards compare at `i64`, not `bool`.

