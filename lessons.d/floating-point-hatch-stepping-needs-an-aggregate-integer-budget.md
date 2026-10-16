---
category: Rust
---

# Floating point hatch stepping needs an aggregate integer budget

Incoming Ishikawa hatching used while offset<=maximum with offset+=5. At finite
canvas1e20/defaultfont, a fixed-operation binary64 audit gives an admitted first
offset whose ULP is16384: adding5 leaves it unchanged. Positive finite font/span
checks also fail to bound the number of hatch scans and resulting Paint paths.
These are source/math witnesses, not an executed Rust hang or UB receipt.

Retain parsed/typed font and text-frame safety fixes, then specify finite signed
grid admission, checked integer iteration, aggregate line/two-pass edge/exact
segment budgets, fallible reservation and no partial output. Count actual pairs
in a bounded first pass instead of rejecting a supported cap-font case through
an unnecessarily conservative pair reservation. Distinguish generated negative
first-grid boundaries and unused positive next offsets. Keep independent exact
ordinary hatch/complete-scene and resource/liveness tests. Repair is still pending
when this lesson is recorded; do not describe a reviewed contract as execution.


Later actual cc911199 compiled liveness red confirms bounded public noncompletion:
zero causes/effectE/HandDrawn/defaultfont/canvas1e20 ran only in the exact owned
test child. The parent observed no completion by3s, killed and reaped that Child
handle at3006ms; Cargo101/buildtrue,1P1F. Receipt/log remain preserved. This is
an executed bounded timeout, not an observation of infinite duration or a Rust
UB claim. No existing process or frozen Miri run was queried or terminated. The
compatibility-wrapper test expects the future explicit unsupported-geometry
panic; it remains red until the checked resource repair exists.
