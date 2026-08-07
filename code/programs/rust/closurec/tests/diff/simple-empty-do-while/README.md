# simple-empty-do-while

End-to-end oracle for **empty-bodied do-while lowering** in
`closure-pass-fold-control-flow` (fcf 0.38.0).

`do {} while(test)` is equivalent to `while(test){}` (the leading empty body is
a no-op), so the empty case lowers to the equivalent loop, which is rewritten to
`for` with the empty body normalized to `;`. A statically-falsy test makes it a
dead loop (removed). A non-empty do-while keeps the `do` form.

## Expected output

```text
for(;cond;);for(;run(););do work();while(again);
```

Byte-identical to the reference Closure Compiler (`v20260712`,
`SIMPLE_OPTIMIZATIONS`, `ECMASCRIPT_2020`, `NO_TRANSPILE`), verified with `xxd`.
