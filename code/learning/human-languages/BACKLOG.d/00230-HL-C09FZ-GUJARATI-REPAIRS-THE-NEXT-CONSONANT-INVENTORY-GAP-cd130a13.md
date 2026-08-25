## HL-C09FZ — Gujarati ઝ repairs the next consonant inventory gap

The canonical-order audit found **ઝ** in both the t30apps.com source and bundled
Noto Sans Gujarati font but absent from `gujarati.json`. Its animation uses
three populated SVG paths: the rounded left body, a separate right loop-and-tail
run, and a final short descending upper stem. The restored entry and fitted
medians preserve that order and the two observed lifts.

Gujarati is now **20/39 verified, 19 remaining**. Continue in source order with
**ઞ** next; it is also absent from the current inventory. A correctness defect
still outranks coverage if one appears during fitting or validation.

