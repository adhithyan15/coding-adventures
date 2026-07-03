// SIMPLE-level optimization THROUGH a for-of loop (CLOC23).
//
// Before CLOC23, *any* program containing a `for`-`of` loop failed the
// typed-AST parse (ForOfStatement was unrepresentable) and closurec silently
// fell back to WHITESPACE_ONLY — zero optimization. This fixture is the
// end-to-end oracle proving the SIMPLE pipeline now runs every pass inside the
// for-of body and recurses into its iterable expression:
//
//   * `log` is single-use, so the inliner folds `log(1)` into its body
//     `report(1)` and deletes the now-unused declaration.
//   * `1 + 2` is constant-folded to `3` even though it lives inside the loop
//     body.
//   * The `for (const item of items) { … }` survives verbatim — control flow
//     and the loop variable are preserved.
//   * `after()` AFTER the loop stays reachable: a for-of is not a terminator
//     (the iterable may be empty, so the body may run zero times).
function log(p) {
  report(p);
}
log(1);
for (const item of items) {
  var x = 1 + 2;
  use(item, x);
}
after();
