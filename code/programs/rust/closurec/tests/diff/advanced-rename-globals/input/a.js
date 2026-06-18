// ADVANCED-only global renaming (CLOC13.I wiring).
//
// `helper` is a top-level function with a multi-statement body, so the
// `inline` pass leaves it, and it is called, so `treeshake` keeps it.
// Under SIMPLE it survives with its full name (a top-level name might be
// externally visible, so SIMPLE never renames it). Under ADVANCED the
// `rename-globals` pass shortens the private top-level `helper` to `a`,
// rewriting the call site too. `sideEffect` and `value` are free globals
// (not declared here) and are left alone.
//
//   SIMPLE   => function helper(){sideEffect();return value};helper();
//   ADVANCED => function a(){sideEffect();return value};a();
//
// (A `--externs` file declaring `helper` would keep the name under
// ADVANCED too — that boundary is what makes the rename sound.)
function helper() {
  sideEffect();
  return value;
}
helper();
