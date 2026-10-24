---
category: Testing & coverage
---

# Instruction-sequence tests must derive expectations after every intervening architectural write

A System/360 integration test initialized a register for a later character
store, then executed `LM` first and unknowingly replaced that register. Other
expectations similarly ignored the final XOR in a byte sequence and a later
floating sign-control instruction that changed the condition code. The
simulator produced the sequentially correct results, but several assertions
claimed the earlier state.

For a multi-instruction witness, annotate the state transition after every
order before writing the final assertions. When one test is intended to prove
two independent control-flow paths, use two short programs so a successful
branch cannot make the second witness unreachable.
