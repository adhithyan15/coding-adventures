// A `SequenceExpression` (the comma operator) `(a, 1 + 2)` (CLOC12.160 PR2)
// now flows through the full SIMPLE pipeline (parser -> typed-AST bridge ->
// passes -> emitter) instead of declining at the bridge and dragging the whole
// file to WHITESPACE_ONLY (gap-161, now closed). It sits as the sole argument
// to `log(...)`, where a bare comma would otherwise be read as a second
// argument — so it MUST round-trip parenthesised. Two facts prove the pipeline
// ran end-to-end: the sequence `(a, 3)` round-trips as a single wrapped
// argument (the bridge produced a real SequenceExpression rather than
// declining), AND its second operand `1 + 2` folds to `3`. A WHITESPACE_ONLY
// fallback (which a bridge decline forces) would re-emit the source verbatim,
// leaving `1 + 2` unfolded.
log((a, 1 + 2));
