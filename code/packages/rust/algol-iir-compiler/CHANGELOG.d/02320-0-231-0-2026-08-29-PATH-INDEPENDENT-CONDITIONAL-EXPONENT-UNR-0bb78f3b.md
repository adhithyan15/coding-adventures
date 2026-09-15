## 0.231.0 — 2026-08-29 — path-independent conditional exponent unrolling

Real-base power unrolling now accepts pure conditional exponent expressions
whose exact tracked integer branches prove one path-independent bounded value.
The selector still lowers and executes in source order; effectful selectors,
differing branches, invalid metadata, and oversized results retain `f64_pow`.

