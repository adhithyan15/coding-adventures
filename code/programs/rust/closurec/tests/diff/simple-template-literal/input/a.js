// A no-substitution template literal (CLOC12.155) flows through the full SIMPLE
// pipeline (parser → typed-AST bridge → passes → emitter) instead of declining
// at the bridge and dragging the whole file to WHITESPACE_ONLY. As of CLOC12.197
// constant-fold also collapses it to a plain string literal (`"hello"`),
// matching the reference Closure Compiler. Both facts are observable below: the
// template folds to `"hello"`, and the adjacent `1 + 2` folds to `3` — a
// WHITESPACE_ONLY fallback (which a bridge decline would force) would leave both
// `` `hello` `` and `1+2` intact and unfolded.
log(`hello`, 1 + 2);
