// SIMPLE does not inline top-level functions (open-world); function inlining
// is ADVANCED-only (CLOC13.G originally inlined at SIMPLE; that was an
// open-world miscompile and is now reverted).
//
// Substituting a top-level function's body into its call sites — and then
// dropping the now-unreferenced declaration — rewrites an observable global,
// so it is a CLOSED-WORLD transform that runs ONLY at ADVANCED. At SIMPLE the
// function and every call are left verbatim:
//
//   `sq` is KEPT; `a(sq(3))` and `b(sq(4))` stay as calls (the arithmetic
//   `3 * 3` / `4 * 4` never appears, because the body is not substituted).
//
// Result: `function sq(x){return x*x};a(sq(3));b(sq(4));`. Under ADVANCED the
// body is inlined at both sites, `sq` is tree-shaken, and constant-fold folds
// `3 * 3` → 9 and `4 * 4` → 16, giving `a(9);b(16);`.
function sq(x) {
  return x * x;
}
a(sq(3));
b(sq(4));
