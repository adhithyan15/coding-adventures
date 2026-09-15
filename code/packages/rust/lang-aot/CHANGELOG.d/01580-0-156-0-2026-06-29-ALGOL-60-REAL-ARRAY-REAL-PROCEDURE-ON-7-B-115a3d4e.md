## 0.156.0 — 2026-06-29 — ALGOL 60 real array + real procedure on 7 backends (LANG-FULL AL9)

Added two matrix proof cells for ALGOL 60 real-typed features:

- **AL9-a (real array)**: `real array A[1:3]` with f64 element stores/loads;
  `A[1]:=40.0; A[3]:=2.0; entier(A[1]+A[3])` ⇒ exit 42.  Exercises non-contiguous
  f64 slots and ALGOL's lower-bound subtraction on array access.
- **AL9-b (real procedure)**: `real procedure square(x); value x; real x;` with
  a real value parameter and real return; `entier(square(6.5))` = `entier(42.25)`
  ⇒ exit 42.  Exercises the `(f64) → f64` call/ret pathway on all 7 backends.

No new IIR ops or backend changes — both cells run on the existing E5 array substrate
(array_set/array_get with f64 type_hint) and the existing call/ret mechanism.

844 total proven cells pass.

