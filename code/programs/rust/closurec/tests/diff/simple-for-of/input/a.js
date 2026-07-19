// SIMPLE-level optimization THROUGH a for-of loop (CLOC23).
//
// Before CLOC23, *any* program containing a `for`-`of` loop failed the
// typed-AST parse (ForOfStatement was unrepresentable) and closurec silently
// fell back to WHITESPACE_ONLY — zero optimization. This fixture is the
// end-to-end oracle proving the SIMPLE pipeline now runs every pass inside the
// for-of body and recurses into its iterable expression:
//
//   * `1 + 2` is constant-folded to `3` even though it lives inside the loop
//     body.
//   * `function log` is KEPT and `log(1)` stays a call. SIMPLE is
//     open-world: it never inlines or deletes an observable top-level name
//     (another script sharing the page could call `log`). The single-use
//     inline that would fold `log(1)` into `report(1)` runs only at
//     ADVANCED (closed-world).
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
