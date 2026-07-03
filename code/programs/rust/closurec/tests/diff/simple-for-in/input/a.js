// SIMPLE-level optimization THROUGH a for-in loop (CLOC22).
//
// Before CLOC22, *any* program containing a `for`-`in` loop failed the
// typed-AST parse (ForInStatement was unrepresentable) and closurec silently
// fell back to WHITESPACE_ONLY — zero optimization. This fixture is the
// end-to-end oracle proving the SIMPLE pipeline now runs every pass inside the
// for-in body and recurses into its right-hand expression:
//
//   * `log` is single-use, so the inliner folds `log(1)` into its body
//     `report(1)` and deletes the now-unused declaration.
//   * `1 + 2` is constant-folded to `3` even though it lives inside the loop
//     body.
//   * The `for (var key in obj) { … }` survives verbatim — control flow and
//     the loop variable are preserved.
//   * `after()` AFTER the loop stays reachable: a for-in is not a terminator
//     (the body may run zero times).
function log(p) {
  report(p);
}
log(1);
for (var key in obj) {
  var x = 1 + 2;
  use(key, x);
}
after();
