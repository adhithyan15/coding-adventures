## 0.5.0 — One-dimensional arrays (LANG-FULL enabler E5 / AL2)

Array declarations and subscripts were rejected ("array variables/subscripts").
They now lower to the IIR's E5 array primitive (`interpreter-ir` 0.7.0), which
`vm-core` 0.7.0 executes on a bounds-checked heap:

- **`integer array A[1:10]`** (and `real array`) → an `alloc_array` whose length
  is the **run-time** span `upper - lower + 1`. ALGOL's *dynamic* bounds
  (`array A[lo:hi]` with expression bounds) work because the bounds are emitted
  as ordinary integer expressions, not folded constants. The binding records the
  lower bound so subscripts can be translated.
- **`A[i]` in an expression** → `array_get`, with the index translated to the
  IIR's **0-based** form `i - lower` (ALGOL arrays are declared with an explicit,
  often 1-based, lower bound).
- **`A[i] := e`** → `array_set`, same index translation, with `e`'s type checked
  against the element type.
- **Bounds-checked by construction**: an out-of-range subscript traps at run time
  (`vm-core` returns a `VMError`, surfaced as `CompileError::Runtime`).
- A segment with several names (`integer array A, B[1:2]`) declares **distinct,
  non-aliasing** arrays sharing one set of bounds.

Scope (this slice): **1-D**, with `integer`/`real` element types. Multidimensional
arrays (`M[i, j]`), non-numeric element types, and arrays as procedure parameters
produce a clear "unsupported" message and are tracked as follow-up. Verified end
to end by 9 new unit tests (store/load round-trip, 1-based and non-unit lower
bounds, fill-and-sum in `for` loops, distinct-array segments, out-of-bounds trap,
scalar-subscript and 2-D rejections, real arrays) plus a `lang-aot` matrix `Prog`
that runs a sum-of-squares array program on **VM + JIT** (exit 55). The code-gen
backends lower the array ops in E5 PR-3 (managed) and PR-4 (static).

