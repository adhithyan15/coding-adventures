// SIMPLE-level do-while loop-body comma-fusion (fold-control-flow 0.36.0),
// the do-while counterpart of the for/while fusion (0.32.0).
//
// A do-while body that is a block of all-plain-expression statements fuses to a
// single (possibly comma-sequenced) expression statement, dropping the braces:
//   do { a(); b(); } while (c);   -> do a(),b();while(c);
// The comma operator runs the operands left-to-right with identical side effects
// and the loop discards the value, so the rewrite is behaviour-preserving.
//
// It runs AFTER the body's own inner folds, so `if (x) g();` that folded to
// `x&&g()` participates:
//   do { if (x) g(); h(); } while (d);  -> do x&&g(),h();while(d);
//
// A body carrying a `var` declaration is NOT fused (a declaration can't join a
// comma-sequence); the block is kept intact:
//   do { var v = 1; k(v); } while (e);  -> do{var v=1;k(v)}while(e);
do { a(); b(); } while (c);
do { if (x) g(); h(); } while (d);
do { var v = 1; k(v); } while (e);
