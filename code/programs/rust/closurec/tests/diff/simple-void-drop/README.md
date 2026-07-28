# simple-void-drop

End-to-end oracle for **`void`-operator drop in statement position** in
`closure-pass-dce` (0.30.0).

`void <impure>;` as an expression statement drops the redundant `void` wrapper,
keeping the operand (`void f();` -> `f();`). A `void` whose result is observed
(non-statement position, e.g. `h(void g())`) is kept. A pure `void <lit>;` is
declined here (a pass-ordering follow-up).

## Expected output

```text
f();a.b();new C;a();b();h(void g());
```

Byte-identical to the reference Closure Compiler (`v20260712`,
`SIMPLE_OPTIMIZATIONS`, `ECMASCRIPT_2020`, `NO_TRANSPILE`), verified with `xxd`.
