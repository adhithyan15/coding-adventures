// An update expression `i++` (CLOC12.158 PR2) now flows through the full
// SIMPLE pipeline (parser → typed-AST bridge → passes → emitter) instead of
// declining at the bridge and dragging the whole file to WHITESPACE_ONLY.
// Two facts prove the pipeline ran end-to-end: `i++` round-trips as a postfix
// update (never silently dropped to `i`), AND the adjacent `1 + 2` folds to
// `3` — a WHITESPACE_ONLY fallback (which a bridge decline would force) would
// leave both `i++` untouched in place and `1 + 2` unfolded.
i++;
log(1 + 2);
