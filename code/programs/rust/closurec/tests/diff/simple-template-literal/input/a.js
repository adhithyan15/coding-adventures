// A no-substitution template literal (CLOC12.155) now flows through the full
// SIMPLE pipeline (parser → typed-AST bridge → passes → emitter) instead of
// declining at the bridge and dragging the whole file to WHITESPACE_ONLY.
// Both facts are observable below: the template round-trips as a backtick
// literal, and the adjacent `1 + 2` folds to `3` — a WHITESPACE_ONLY fallback
// (which a bridge decline would force) would leave `1+2` intact and unfolded.
log(`hello`, 1 + 2);
