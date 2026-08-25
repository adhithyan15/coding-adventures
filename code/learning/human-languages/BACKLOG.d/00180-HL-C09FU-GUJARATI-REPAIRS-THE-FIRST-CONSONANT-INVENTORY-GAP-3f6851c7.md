## HL-C09FU — Gujarati ઘ repairs the first consonant inventory gap

The canonical-order audit found **ઘ** in both the t30apps.com source and bundled
Noto Sans Gujarati font but absent from `gujarati.json`. Its animation uses two
populated SVG paths: one continuous run flows through the upper lobe, middle
turn, and rounded lower body; after one lift, a separate run descends the full
right spine and turns through its lower foot. The restored entry and fitted
medians preserve that joined-body-before-spine order.

Gujarati is now **15/37 verified, 22 remaining**. Continue in source order with
**ઙ** next; it is also absent from the current inventory. A correctness defect
still outranks coverage if one appears during fitting or validation.

