# simple-fold-math-fround

End-to-end oracle for static `Math.fround(n)` folding in
`closure-pass-constant-fold` (0.110.0).

`Math.fround(x)` rounds `x` to the nearest float32 and widens back to a double
(`x as f32 as f64`). The fold fires ONLY at a float32 fixed point
(`fround(x) === x`), leaving the value unchanged; a double that fround would
change (`1.1`, `2^24+1`), a non-global receiver (`m.fround`), a `-0` fixed
point, and `NaN`/`Infinity` (which are identifiers, not numeric literals) are
all declined.

## Expected output

```text
var a=1.5,b=-2.5,c=.25,d=16777216,e=Math.fround(1.1),f=Math.fround(16777217),g=m.fround(1.5);report(a,b,c,d,e,f,g);
```

Byte-identical to the reference Closure Compiler (`v20260712`,
`SIMPLE_OPTIMIZATIONS`, `ECMASCRIPT_2020`, `NO_TRANSPILE`), verified with `xxd`.
