---
category: Compiler / VM / language pipeline
---

# Forwarded macro arguments must retain their inner expansion chain

PREP01's first provenance pass assigned every function argument token the new
function invocation's expansion id. That worked for `ID(ONE)`, but failed when
an argument had already been expanded and was forwarded: with `H -> 7`,
`F(x) -> x`, and `OUTER(x) -> F(x)`, `OUTER(H)` lost `H` and reported only
`F -> OUTER`. The emitted value was correct, so ordinary expansion tests passed.

Expansion records are shared by tokens. Reparenting `H` in place would corrupt
earlier loci. Copy only the affected prefix, insert `F` before the shared outer
chain, and memoize copies for repeated arguments. Cap original and copied
records before allocation. Test a forwarded argument through two function
macros and assert every name in the chain. Use a tight node budget to prove
the copying is bounded.
