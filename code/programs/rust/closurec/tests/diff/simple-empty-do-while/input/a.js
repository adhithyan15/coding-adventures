// SIMPLE-level empty-bodied do-while lowering (fold-control-flow 0.38.0).
//
// `do {} while(test)` runs its (empty) body once and then evaluates `test`.
// Because the body is a no-op, the test-evaluation sequence is IDENTICAL to
// `while(test){}`, so the empty case lowers to the equivalent loop, which the
// existing machinery rewrites to `for` (and normalizes the empty body to `;`):
//   do {} while (cond);   -> for(;cond;);
//   do {} while (run());  -> for(;run(););   (impure test preserved)
//
// A NON-empty do-while keeps the `do` form (its body runs before the test, so
// it can't generally become a `while`); a single-statement body just unwraps:
//   do { work(); } while (again);  -> do work();while(again);
do {} while (cond);
do {} while (run());
do { work(); } while (again);
