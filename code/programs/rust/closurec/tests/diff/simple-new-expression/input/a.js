// A `new` expression `new Widget(1 + 2)` (CLOC12.159 PR2) now flows through
// the full SIMPLE pipeline (parser -> typed-AST bridge -> passes -> emitter)
// instead of declining at the bridge and dragging the whole file to
// WHITESPACE_ONLY. It sits as an argument to `log(...)` so the call keeps the
// construction alive. Two facts prove the pipeline ran end-to-end: the
// `new Widget(...)` round-trips (never dropped, and the bridge produced a real
// NewExpression rather than declining), AND the argument `1 + 2` folds to `3`.
// A WHITESPACE_ONLY fallback (which a bridge decline forces) would re-emit the
// source verbatim, leaving `1 + 2` unfolded.
log(new Widget(1 + 2));
