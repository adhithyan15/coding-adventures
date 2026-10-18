---
category: Testing & coverage
---

# Use the full libtest namespace for expected new test identities

A new painter test was included inside `tests::timeline_event_scene_contract`,
but its runner expected identity omitted the outer `tests::`. Both suites
compiled: the102 temporal and190 painter old functions passed, and the four
new functions failed as intended. The identity assertion correctly withheld
aggregate red acceptance. Preserve its actual exits/raw/status maps, correct
the full declared namespace in a successor runner, and rerun on a clean head.
Use the enclosing module declarations and prior full status keys to establish
the expected namespace before native execution; do not weaken exact-name joins.
