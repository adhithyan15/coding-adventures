// SIMPLE-level optimization THROUGH a do-while loop (CLOC20).
//
// Before CLOC20, *any* program containing a `do`-`while` loop failed the
// typed-AST parse (DoWhileStatement was unrepresentable) and closurec
// silently fell back to WHITESPACE_ONLY — zero optimization. This fixture
// is the end-to-end oracle proving the SIMPLE pipeline now runs every pass
// INSIDE the do-while body and recurses into its test:
//
//   * `1 + 2` is constant-folded to `3` even though it lives inside the
//     do-while body.
//   * `function log` is KEPT and `log(1)` stays a call. SIMPLE is
//     open-world: it never inlines or deletes an observable top-level name
//     (another script sharing the page could call `log`). The single-use
//     inline that would fold `log(1)` into `report(1)` runs only at
//     ADVANCED (closed-world).
//   * The `do { … } while (cond)` survives verbatim — control flow is
//     never altered.
//   * `foo()` AFTER the loop stays reachable: a do-while is not a
//     terminator (control can fall out of the loop).
function log(p) {
  report(p);
}
log(1);
do {
  var x = 1 + 2;
  step(x);
} while (cond);
foo();
