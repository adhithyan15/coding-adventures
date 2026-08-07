# simple-empty-loop-body

End-to-end oracle for **empty-loop-body normalization** in
`closure-pass-fold-control-flow` (fcf 0.37.0).

A `for` loop whose body folds to an empty block (`{}`, `{;;}`, `{{}}`) has that
body normalized to an empty statement (`;`). Because `while` is first rewritten
to `for` (0.31.0) and re-folded, the one change covers both `for` and `while`
empty bodies. A non-empty body is unaffected.

## Expected output

```text
for(var i=0;i<n;i++);for(;cond;);for(;run();)step();
```

Byte-identical to the reference Closure Compiler (`v20260712`,
`SIMPLE_OPTIMIZATIONS`, `ECMASCRIPT_2020`, `NO_TRANSPILE`), verified with `xxd`.
