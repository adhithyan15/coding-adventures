---
category: Compiler / VM / language pipeline
---

# A frontend cannot store into a SIR parameter or redeclare it under its own name, so an assigned parameter needs a fresh-named local copy

`python-to-semantic-ir` lowered the first store to a parameter (`n -= 1`,
`n = n - 1`) as a new `LetStarBinding n`.  Inside a loop body that local
ended with the body, so `while n > 0: n -= 1` never changed the `n` the
condition read, and looped forever on every backend (#18482).

Two direct fixes were tried, and each failed on a backend:

- `Assign { name: n, scope: Param }` is valid SIR (the validator accepts
  it), but the Rust backend emits parameters as immutable bindings:
  `error[E0384]: cannot assign to immutable argument`.
- A prologue `let* n = n (Param)` that copies the parameter into a local of
  the **same name** fails on JavaScript (`let n = n;` is a `SyntaxError`
  next to the parameter `n`), and Go and C refuse the same redeclaration.

The fix that works on all six backends: copy the parameter into a local
with a **fresh** name (`__param_n_0`, never spelled in the source) at the
top of the body, and send every read and store of `n` to it.  The
parameter keeps its own name, so keyword calls still match it.

What to do: before a frontend emits `Assign` to a `Param` or `Capture`
scope, or a binding that reuses a parameter's name, run the program on
all six backends (`sir-conformance`).  "The validator accepts it" does not
mean every backend compiles it.
