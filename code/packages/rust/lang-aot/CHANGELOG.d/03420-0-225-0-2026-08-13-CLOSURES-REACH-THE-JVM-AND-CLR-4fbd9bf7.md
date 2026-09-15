## 0.225.0 - 2026-08-13 (closures reach the JVM and CLR)

`lower_closures_to_heap` now runs in the **JVM and CIL** pipelines, not just
WASM. Those backends previously refused a closure outright —
`UnsupportedOp { op: "alloc_closure" }` — and simply wiring the pass in was not
enough: it removed the refusal and returned wrong values instead, which is a
worse state than refusing. The representation bug behind that is fixed in
iir-builtin-lowering 0.38.0 (a pass may box a value crossing a boundary; it may
not redefine what the boundary is).

Eight matrix cells go green: Twig closures on Clr/Jvm/Wasm for both
`((lambda (x) (+ x 1)) 41)` and the capturing
`(((lambda (x) (lambda (y) (+ x y))) 40) 2)`, plus two more that the same
boundary defect was breaking. The executed matrix drops from 12 confirmed
failures to 4 — all four are pre-existing and tracked (ALGOL string results on
NativeAot/Clr, and the COBOL `COMPUTE` int32 literal on Clr/Jvm).

Verified with a full `LANG_MATRIX_REPORT_ALL` sweep (every failure re-verified in
a fresh process), the McCarthy nine-backend conformance capstone, and all 41
lang-aot test targets.
