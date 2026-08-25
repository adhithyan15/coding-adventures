## HL-C09GK — Gujarati ન restores its loop, shoulder, and tall spine

The source-and-font audit found **ન** missing from `gujarati.json`. The
t30apps.com animation gives it two populated SVG paths: one run circles the
small left loop and continues right across the long shoulder; after one lift,
the separate tall right spine descends into its lower foot. The restored entry
and fitted Noto Sans Gujarati medians preserve that order and the observed lift.

Gujarati is now **31/44 verified, 13 remaining**. Continue in source order with
**પ** next; it is also missing from the current inventory and therefore
outranks later conventional placeholders. A correctness defect still outranks
coverage if one appears during fitting or validation.

The Gujarati-bearing `script-data` batch must remain below its 250 kB authored-
batch target. The production build measures it at **231.24 kB** after this
addition. Split the batch rather than weakening learner-facing provenance if
this or a later entry crosses the target.

