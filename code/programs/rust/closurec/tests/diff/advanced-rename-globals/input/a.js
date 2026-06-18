// ADVANCED-only global renaming (CLOC13.I wiring).
//
// `helper` is a top-level function with a multi-statement body, so the
// expression `inline` pass leaves it; it is called MORE THAN ONCE, so the
// single-use void statement-inliner (CLOC15) also declines it, and
// `treeshake` keeps it (it is referenced). Under SIMPLE it survives with
// its full name (a top-level name might be externally visible, so SIMPLE
// never renames it). Under ADVANCED the `rename-globals` pass shortens the
// private top-level `helper` to `a`, rewriting the call sites too.
// `sideEffect` and `value` are free globals (not declared here) and are
// left alone.
//
//   SIMPLE   => function helper(){sideEffect();return value};helper();helper();
//   ADVANCED => function a(){sideEffect();return value};a();a();
//
// (A `--externs` file declaring `helper` would keep the name under
// ADVANCED too — that boundary is what makes the rename sound. The second
// call is what keeps `helper` from being inlined away entirely — a single
// call site would be spliced in place by the CLOC15 statement-inliner.)
function helper() {
  sideEffect();
  return value;
}
helper();
helper();
