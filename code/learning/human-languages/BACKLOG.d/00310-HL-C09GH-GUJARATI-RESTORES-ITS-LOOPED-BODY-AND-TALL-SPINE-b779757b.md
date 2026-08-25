## HL-C09GH — Gujarati થ restores its looped body and tall spine

The source-and-font audit found **થ** missing from `gujarati.json`. The
t30apps.com animation gives it two populated SVG paths: one continuous run
circles the small upper loop, descends through the middle, sweeps around the
broad lower body, and rises into the right shoulder; after one lift, a separate
tall spine descends into its lower foot. The restored entry and fitted Noto Sans
Gujarati medians preserve that order and the observed lift.

Gujarati is now **28/43 verified, 15 remaining**. Continue in source order with
**દ** next; it is present in the inventory but still has only a conventional
placeholder. A correctness defect still outranks coverage if one appears
during fitting or validation.

The Gujarati-bearing `script-data` batch is **226.28 kB** in this production
build, below its 250 kB authored-batch target. Recheck it on every source note;
split the batch rather than weakening learner-facing provenance if a later
entry crosses the target.

