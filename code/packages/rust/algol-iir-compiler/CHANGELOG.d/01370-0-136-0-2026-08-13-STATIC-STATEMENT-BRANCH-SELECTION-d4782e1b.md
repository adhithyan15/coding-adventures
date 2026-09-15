## 0.136.0 — 2026-08-13 — static statement branch selection

Conditional statements with a statically known condition now carry only the
reachable branch's initialization and scalar snapshot state through the join.
Unknown conditions continue to intersect both exits conservatively.

