## HL-C09GD — Gujarati ડ repairs the adjacent retroflex inventory gap

The canonical-order audit found **ડ** in both the t30apps.com source and bundled
Noto Sans Gujarati font but absent from `gujarati.json`; the prior backlog note
incorrectly called it present. Its animation uses one populated SVG path: the
pen begins at the upper right, sweeps left across the high shoulder, descends
through the middle and around the broad lower bowl, then finishes at the lower
left without lifting. The restored entry and fitted median preserve that order.

Gujarati is now **24/42 verified, 18 remaining**. Continue in source order with
**ઢ** next; it is present in the current inventory but still needs cited ductus
verification. A correctness defect still outranks coverage if one appears
during fitting or validation.

The Gujarati-bearing `script-data` batch is **219.74 kB** in this production
build, below its 250 kB authored-batch target. Recheck it on every added source
note; split the batch rather than weakening learner-facing provenance if a
later entry crosses the target.

