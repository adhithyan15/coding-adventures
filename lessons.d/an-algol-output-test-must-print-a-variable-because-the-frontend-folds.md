---
category: Testing & coverage
---

# An ALGOL output test must print a variable, because the frontend folds an integer literal into a string

**What went wrong.** The LANG83 ordered-output row was meant to mix ALGOL's two
output paths: an integer `output(…)` lowers to `call_builtin "print_i64"` (value
plus newline), and a string `print(…)` lowers to `print_str`. The first draft was
`begin print('A'); output(1); print('B'); output(-2); print('C') end`, expecting
`A1\nB-2\nC`. Native printed `A1B-2\nC`. The frontend folds `output(1)` into
`str_const "1"` + `print_str`, so there is no newline and no `print_i64` at all.
`output(-2)` is computed at run time (`sub 0, 2`), so it alone reached
`print_i64`. The test was not testing the builtin it was named for.

**Fix.** Print a variable (`integer n; n := 1; … output(n); … output(n - 3)`),
which forces `print_i64`. Dumping the module first (`compile_source_to_iir` and
print each instruction) showed the folding in one line.

**Next time.** Before writing a source-level test whose point is a particular IIR
op, dump the frontend's IIR for the exact program and confirm the op is there.
Literal operands are where frontends constant-fold, so prefer runtime values. If
the claim is about the op itself, build the IIR directly, as
`lang-aot/tests/op_conformance.rs` does.
