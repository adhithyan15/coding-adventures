## HL-C09GC — Gujarati ઠ closes the next consonant inventory gap

The canonical-order audit found **ઠ** in both the t30apps.com source and bundled
Noto Sans Gujarati font but absent from `gujarati.json`. Its animation uses one
populated SVG path: the pen begins at the upper right, sweeps left across the
high shoulder, descends through the middle and around the broad outer lower
bowl, then curls back inward to its terminal without lifting. The restored
inventory entry and fitted median preserve that order and zero-lift evidence.

Gujarati is now **23/41 verified, 18 remaining**. Continue in source order with
**ડ** next; it is present in the source and current inventory but still needs
cited ductus verification. A correctness defect still outranks coverage if one
appears during fitting or validation.

The Gujarati-bearing `script-data` batch is **217.89 kB** in this production
build, below its 250 kB authored-batch target. Keep rechecking it as the cited
inventory grows; split the batch rather than weakening or deleting learner-
facing provenance if a later entry crosses the target.

