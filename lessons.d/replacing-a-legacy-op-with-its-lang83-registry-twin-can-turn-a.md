---
category: Compiler / VM / language pipeline
---

# Replacing a legacy op with its LANG83 registry twin can turn a wrong answer into a compile failure, because the registry op's backend validators demand typed operands

**What went wrong.** While fixing #18470, `iir-builtin-lowering`'s
`dynamic_arith` table turned out to map Twig's `modulo` to the truncating
`mod`, so `(define (f x y) (modulo x y)) (f -7 2)` answered −1 instead of 1
on WASM. The one-line fix — map `modulo` to the registry's floored
`i64_mod_floor`, which every backend's `op_support` declares `Lowered` —
made the same program **fail to compile** on WASM:
`InvalidOperand { detail: "i64_mod_floor operates on i64 operands and an i64
dest" }`. Twig's un-annotated parameters reach that op typed `any`. The
legacy `mod` lowering tolerated `any`; the LANG83 PR1b op's validator does
not. JVM refused the same function too (`UnsupportedType`). The unit test on
the pass's output passed, because it checked only which op was emitted.

**Fix.** The change was reverted. On the VM and JIT columns, `vm_lisp` maps
`modulo` to `i64_mod_floor` itself (the VM accepts any integer `Value`), and
the WASM `modulo` bug is recorded on the #18470 PR as found-along-the-way.

**Do differently.** `op_support` says *whether* a backend lowers an op, not
*which operand types* it accepts. Before swapping a legacy op for its
registry twin in a pass that runs on every backend, run a program that
reaches it with dynamically-typed operands (an un-annotated Twig parameter)
on every local column, not just the pass's unit tests.
