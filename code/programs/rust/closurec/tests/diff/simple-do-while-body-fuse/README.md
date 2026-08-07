# simple-do-while-body-fuse

End-to-end oracle for **do-while loop-body comma-fusion** in
`closure-pass-fold-control-flow` (fcf 0.36.0) — the do-while counterpart of the
`for`/`while` fusion (0.32.0).

A do-while body that is a block of all-plain-expression statements fuses to a
single (possibly comma-sequenced) expression statement, dropping the braces. It
runs after the body's inner folds (so a folded `if` participates), and declines
on any body carrying a declaration or a control-flow statement.

## Expected output

```text
do a(),b();while(c);do x&&g(),h();while(d);do{var v=1;k(v)}while(e);
```

Byte-identical to the reference Closure Compiler (`v20260712`,
`SIMPLE_OPTIMIZATIONS`, `ECMASCRIPT_2020`, `NO_TRANSPILE`), verified with `xxd`.
