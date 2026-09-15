## 0.222.5 - 2026-08-12 (ALGOL conditional while predicates — seven backends)

The ALGOL initial-`while` matrix cell now runs a statically selected
conditional boolean predicate on all seven standard backends. LLVM now stores
the comparison's `i1` sidecar, rather than its widened `i64`, at the branch join.

