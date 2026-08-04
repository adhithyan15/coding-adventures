// SIMPLE-level optimization THROUGH a try/catch/finally (CLOC19).
//
// Before CLOC19, *any* program containing `try` failed the typed-AST
// parse (TryStatement was unrepresentable) and closurec silently fell
// back to WHITESPACE_ONLY — zero optimization. This fixture is the
// end-to-end oracle proving the SIMPLE pipeline now runs every pass
// INSIDE the try block, the catch handler, and the finally block:
//
//   * `1 + 2` and `3 * 4` are constant-folded to `3` and `12` even
//     though they live inside the try/catch blocks.
//   * `function log` is KEPT and `log(1)` stays a call. SIMPLE is
//     open-world: it never inlines or deletes an observable top-level name
//     (another script sharing the page could call `log`). The single-use
//     inline that would fold `log(1)` into `report(1)` runs only at
//     ADVANCED (closed-world).
//   * The `dead(99)` call after the `return` in the catch handler is
//     unreachable, so block-level DCE truncates it.
//   * `try`, `catch (e)`, and `finally` survive verbatim — control
//     flow is never altered; the catch binding `e` is preserved.
function log(p) {
  report(p);
}
log(1);
try {
  var x = 1 + 2;
  risky(x);
} catch (e) {
  var y = 3 * 4;
  handle(e, y);
  return;
  dead(99);
} finally {
  cleanup();
}
