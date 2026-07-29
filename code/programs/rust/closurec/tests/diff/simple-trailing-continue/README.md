# simple-trailing-continue

End-to-end oracle for **trailing-`continue` removal** in
`closure-pass-fold-control-flow` (fcf 0.39.0).

A bare (unlabeled) `continue` at the tail of a for/while/do-while body is removed
(it is a no-op); the shortened body then unwraps or normalizes. Labeled
continues and continues with dead code after them are left intact.

## Expected output

```text
for(;c;)step();for(;d;)work();do tick();while(e);for(;f;);
```

Byte-identical to the reference Closure Compiler (`v20260712`,
`SIMPLE_OPTIMIZATIONS`, `ECMASCRIPT_2020`, `NO_TRANSPILE`), verified with `xxd`.
