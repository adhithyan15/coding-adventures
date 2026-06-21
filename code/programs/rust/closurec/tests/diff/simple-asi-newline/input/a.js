// CLOC26 Phase 2 — Automatic Semicolon Insertion at a line terminator.
//
// This program has NO semicolons at all; each statement sits on its own line.
// ECMAScript inserts a semicolon before a token that is preceded by a line
// terminator (ASI Rule 1), so `var w = 4` and `var s = 1 + 2` are separate
// statements. Before Phase 2 the grammar parser required explicit semicolons,
// so this whole program failed to parse and closurec degraded to
// WHITESPACE_ONLY. ASI now supplies the semicolons at the line breaks, the
// program parses, and the SIMPLE pipeline folds `1 + 2` to `3`.
//
// At SIMPLE this becomes:
//   var w=4;var s=3;report(w * s);
// while WHITESPACE_ONLY keeps `1+2` verbatim (it runs no passes).
var w = 4
var s = 1 + 2
report(w * s)
