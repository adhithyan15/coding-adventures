# simple-spread-array-flatten

End-to-end oracle for **array-literal spread flattening** in
`closure-pass-constant-fold` (0.111.0).

A `...[…]` whose argument is a hole-free array literal is inlined into the
enclosing array literal (`[...[1,2],3]` -> `[1,2,3]`). Non-literal spreads
(`[...y,4]`) and hole-carrying inner literals are left intact.

## Expected output

```text
var a=[1,2,3],b=[0,1,2,3],c=[1,2,3],d=[...y,4];
```

Byte-identical to the reference Closure Compiler (`v20260712`,
`SIMPLE_OPTIMIZATIONS`, `ECMASCRIPT_2020`, `NO_TRANSPILE`), verified with `xxd`.
