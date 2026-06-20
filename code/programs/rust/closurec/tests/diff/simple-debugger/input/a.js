// SIMPLE-level optimization of a program containing `debugger;` (CLOC21).
//
// Before CLOC21, *any* program containing a `debugger` statement failed the
// typed-AST parse (DebuggerStatement was unrepresentable) and closurec
// silently fell back to WHITESPACE_ONLY — zero optimization. This fixture is
// the end-to-end oracle proving the SIMPLE pipeline now runs across a
// `debugger` statement, optimizing the rest of the program while preserving
// the statement verbatim:
//
//   * `log` is single-use, so the inliner folds `log(1)` into its body
//     `report(1)` and deletes the now-unused declaration.
//   * `1 + 2` is constant-folded to `3`.
//   * `debugger;` survives verbatim — v1 preserves it (stripping it, as the
//     upstream Closure Compiler does, is future work).
function log(p) {
  report(p);
}
log(1);
var x = 1 + 2;
debugger;
use(x);
