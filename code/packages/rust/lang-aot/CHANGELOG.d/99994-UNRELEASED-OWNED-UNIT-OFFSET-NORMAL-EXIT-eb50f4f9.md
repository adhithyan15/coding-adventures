## [Unreleased] — execute owned unit offsets with normal completion

Add12 saved-value programs covering sign -1/0/+1, +/-1 offsets, direct/saved
roots, reversed operands/unary signs and both selector branches. Assert exact
stdout and Exit0 for each available standard backend and require at least one
execution per source. Keep all original matrix/availability checks unchanged.
Actual focused evidence:72 successful runs across six backends, one test target;
CLR is unavailable. Full final unfiltered suites/strict remain required and the
cancelled4a baseline stays incomplete. Miri DEFERRED/PENDING, never passed.
