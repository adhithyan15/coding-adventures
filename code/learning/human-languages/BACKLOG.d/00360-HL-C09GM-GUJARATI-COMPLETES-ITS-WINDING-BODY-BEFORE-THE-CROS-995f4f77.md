## HL-C09GM — Gujarati ફ completes its winding body before the cross-stroke

The source-and-font audit found **ફ** missing from `gujarati.json`. The
t30apps.com animation gives it two populated SVG paths: one continuous run
sweeps left across the high cap, winds down and around the body, circles the
small lower-left loop, and exits through the descending tail; after one lift,
a separate diagonal cross-stroke rises from lower left to upper right. The
restored entry and fitted Noto Sans Gujarati medians preserve that order.

Gujarati is now **33/46 verified, 13 remaining**. Continue in source order with
**બ** next; it is also missing from the current inventory and therefore
outranks later conventional placeholders. A correctness defect still outranks
coverage if one appears during fitting or validation.

The Gujarati-bearing `script-data` batch must remain below its 250 kB authored-
batch target. The production build measures it at **235.05 kB** after this
addition. Split the batch rather than weakening learner-facing provenance if
this or a later entry crosses the target.

