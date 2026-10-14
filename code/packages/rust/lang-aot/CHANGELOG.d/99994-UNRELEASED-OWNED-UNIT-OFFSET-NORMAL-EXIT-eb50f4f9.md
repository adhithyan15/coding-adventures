## [Unreleased] — execute owned unit offsets with normal completion

Add12 saved-value programs covering sign -1/0/+1, +/-1 offsets, direct/saved
roots, reversed operands/unary signs and both selector branches. Assert exact
stdout and Exit0 for each available standard backend and require at least one
execution per source. Keep all original matrix/availability checks unchanged.
Actual focused evidence:72 successful runs across six backends, one test target;
CLR is unavailable. Final5c full20-package unfiltered suites pass2613/0/1existing
ignore over110targets (LANG478/0), strict alltargets EXIT0; the
cancelled4a baseline stays incomplete. Miri DEFERRED/PENDING, never passed.
