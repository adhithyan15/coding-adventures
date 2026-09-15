## 0.148.0 — 2026-08-13 — static while-control exits

A self-contained `while` for-list element whose value and predicate reference
only its controlled scalar now retains the first control value whose predicate
is false. Abstract execution is capped at 4,096 iterations; external
dependencies, unknown expressions, and nonterminating or longer loops remain
conservative.

