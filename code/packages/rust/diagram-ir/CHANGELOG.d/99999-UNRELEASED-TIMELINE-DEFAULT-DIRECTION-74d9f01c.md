## Unreleased — consistent Timeline default direction

Bare `timeline` and `TimelineDirection::default()` now select left-to-right;
explicit TD and LR keep their existing behavior. Configuration ownership,
preprocessing, checked geometry and existing sources/assertions are retained.
At `74d9f01c`, all 1168 native tests passed across 55 harnesses
(0 existing ignored), with 2 doctests passed/0 ignored
across 14 targets and all 14 strict checks passing. Complete new controls cover
37 source/empty/retained IR cells and three portable IR/layout/Scene/query cells.
This is bounded progress, unmerged pending current review/conflict/hosted gates.
Apple rendering remains unexecuted here. Miri DEFERRED/PENDING, never passed;
full Closure parity and transformation lineage remain OPEN.
