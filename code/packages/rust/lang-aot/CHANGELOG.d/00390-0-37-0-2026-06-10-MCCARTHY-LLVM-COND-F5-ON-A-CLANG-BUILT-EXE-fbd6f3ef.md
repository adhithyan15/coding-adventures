## 0.37.0 — 2026-06-10 — McCarthy → **LLVM `COND`** (F5) on a clang-built executable — LLVM core F1–F5 (LANG77 / W12b-3)

No `lang-aot` source change — the work is in `iir-to-llvm` 0.6.0 (cross-block
SSA-merge via stack-slot/`alloca` promotion, plus the `jmp_if` void-cond and
empty-block fallthrough fixes), which `compile_source_to_llvm` already drives. New
verify-by-running test `tests/llvm_cond.rs` (link `lispy_runtime.c` with clang,
run): `(COND ((ATOM 7) 11) ((ATOM 8) 22))`→11, second-clause→22, nested `COND`→44;
cons/predicate/scalar all still pass. **LLVM is now F1–F5 (only symbols+lambda,
W13, remain).**

