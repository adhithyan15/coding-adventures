// SIMPLE keeps a single-use top-level function (open-world); inline-driven
// fixed-point collapse is ADVANCED-only.
//
// The pass pipeline runs to a FIXED POINT — it re-sweeps the pass order while
// any FixedPoint pass still reports a change, so a transform one pass exposes
// is picked up by an earlier pass on the next sweep. Inlining a top-level
// function is the classic trigger, but it rewrites an observable global, so it
// runs ONLY at ADVANCED. At open-world SIMPLE, `double` is a single-use global
// that is nonetheless KEPT, and `double(7)` is left as a call — no inline, so
// no `7 * 2` for a later sweep to fold:
//
//   Result: `function double(x){return x*2};log(double(7));`.
//
// Under ADVANCED the fixed point does its work: sweep 1 inlines `double(7)` to
// `7 * 2` and tree-shakes `double`; sweep 2 constant-folds `7 * 2` → `14`,
// giving `log(14);`. The fixed-point interplay that DOES survive at SIMPLE —
// `inline-variables` exposing a later `constant-fold` — is exercised by the
// `simple-inline-variables` fixture.
function double(x) {
  return x * 2;
}
log(double(7));
