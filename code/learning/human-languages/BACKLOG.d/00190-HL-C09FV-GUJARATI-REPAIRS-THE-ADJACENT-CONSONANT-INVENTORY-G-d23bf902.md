## HL-C09FV — Gujarati ઙ repairs the adjacent consonant inventory gap

The canonical-order audit found **ઙ** in both the t30apps.com source and bundled
Noto Sans Gujarati font but absent from `gujarati.json`. Its animation uses two
populated SVG paths: one long S-like run flows from the upper-right through the
upper turn and rounded lower body to the lower-left; after one lift, a compact
separate loop draws the upper-right dot. The restored entry and fitted medians
preserve that body-before-dot order.

Gujarati is now **16/38 verified, 22 remaining**. With the adjacent consonant
inventory gaps repaired, continue in source order with existing **ચ** next. A
correctness defect still outranks coverage if one appears during validation.

