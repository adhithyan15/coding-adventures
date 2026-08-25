## HL-C09GL — Gujarati પ joins its high left hook to the lower body

The source-and-font audit found **પ** missing from `gujarati.json`. The
t30apps.com animation gives it two populated SVG paths: one run curls through
the high left hook, descends the left stem, and sweeps around the broad lower
body into the right shoulder; after one lift, the separate tall right spine
descends into its lower foot. The restored entry and fitted Noto Sans Gujarati
medians preserve that order and the observed lift.

Gujarati is now **32/45 verified, 13 remaining**. Continue in source order with
**ફ** next; it is also missing from the current inventory and therefore
outranks later conventional placeholders. A correctness defect still outranks
coverage if one appears during fitting or validation.

The Gujarati-bearing `script-data` batch must remain below its 250 kB authored-
batch target. The production build measures it at **233.10 kB** after this
addition. Split the batch rather than weakening learner-facing provenance if
this or a later entry crosses the target.

