---
category: Testing & coverage
---

# LCOV branch records with a dash are uncovered and remain in the denominator

A Dart package coverage checker skipped `BRDA` records whose taken field was
`-`. The report then omitted those branches from its denominator, so an
uncovered branch could make a threshold look better. Count every branch
record; a dash contributes zero hits. Pin that interpretation with a synthetic
LCOV fixture whose only uncovered branch leaves coverage exactly at the strict
threshold, then require the gate to reject it.
