// CLOC26 Phase 1 — Automatic Semicolon Insertion before `}` / end-of-input.
//
// `area`'s body omits the semicolon after `return w * s` — there is no `;`
// before the closing `}`. Before ASI, the grammar parser required an explicit
// `SEMICOLON` there, so this whole program failed to parse and closurec
// silently degraded to WHITESPACE_ONLY (no optimization at all). ASI now
// inserts the missing `;` before the `}`, the program parses, and the SIMPLE
// pipeline runs: `1 + 2` constant-folds to `3`.
//
// (The inner `var s = 1 + 2;` keeps its explicit semicolon — Phase 1 only
// supplies semicolons before `}` / EOF, not between statements on the same
// line; that statement is followed by `return`, not a `}`.)
//
// At SIMPLE this becomes:
//   function area(w){var s=3;return w*s}report(area(10));
// while WHITESPACE_ONLY keeps `var s=1+2` verbatim (it runs no passes).
function area(w) {
  var s = 1 + 2;
  return w * s
}
report(area(10));
