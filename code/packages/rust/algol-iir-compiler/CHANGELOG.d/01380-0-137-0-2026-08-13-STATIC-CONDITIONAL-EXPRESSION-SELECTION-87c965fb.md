## 0.137.0 — 2026-08-13 — static conditional expression selection

Numeric conditional assignments with a statically known selector now retain
the reachable branch's integer or real snapshot. Unknown selectors continue to
require equal branch values before preserving metadata.

