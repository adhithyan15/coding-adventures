## 0.79.0 — 2026-08-11 — declaration-scoped nested switches

Switch declarations now receive stable lexical identities in a block pre-pass.
Stored switch-list references bind to the declaration-time identity, so a later
nested declaration with the same spelling cannot retarget an outer switch graph.
The pre-pass also preserves forward switch references within one block.

