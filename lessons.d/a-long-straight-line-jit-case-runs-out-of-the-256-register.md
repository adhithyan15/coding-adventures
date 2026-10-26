---
category: Compiler / VM / language pipeline
---

# A long straight-line JIT case runs out of the 256-register file, because constant propagation gives every literal use its own register

**What went wrong.** LANG83 PR7b's conformance cases print `f64_class` (and
`f64_digits`, `f64_exp10`) of 87 table values, one row each:
`const_f64 x_i; f64_class r_i <- x_i; write_i64 r_i; putchar nl`. On the JIT
column `JITCore::compile` returned `Ok(false)`, and `run_jit` failed the case
("uses only JIT-Lowered ops, so `main` must be JIT-compiled"). A one-row
version compiled fine. `GenericCirJit` has 256 `i64` registers. The CIR
optimiser propagates every constant into its use, and the JIT then
materialises each literal operand (`Float(1.5)`, `putchar`'s `Int(10)`) into
a FRESH register at every use. With a distinct result name per row, each row
cost three registers: 87 × 3 = 261 > 256, so compilation was refused and the
function would have run on the interpreter tier.

**Fix.** The table programs reuse ONE result name (`r`) for every row, since
each result is printed at once. That is two registers per row (the float
literal and `putchar`'s newline). The PR6a writer tables never hit the limit
because they have no result register.

**Do differently.** When a conformance or matrix case is a long straight-line
table, count the JIT registers: one per distinct variable, plus one per
literal *use* after constant propagation (not one per distinct literal).
Keep per-row temporaries to a reused name. If a real frontend produces such
code, the fix belongs in `GenericCirJit` (reuse a register per literal, or
spill), not in the case.
