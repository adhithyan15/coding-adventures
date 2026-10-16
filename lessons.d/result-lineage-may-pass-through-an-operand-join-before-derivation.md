---
category: Testing & coverage
---

# Result lineage may pass through an operand join before derivation

CLOC33's initial CLI assertion required the folded result to directly parent
the array ID and required its creation journal event to be Merge. A single
operand legitimately uses Derive; multiple operands first join, then derive the
replacement. The first focused run had six passing private tests but three
failing CLI harnesses, so no green acceptance followed.

Select the result's own fold record, then traverse its actual ancestry to the
exact array occurrence. Keep exact complete source parents, sibling exclusion,
new_cv ownership and before/after operation order. Allow Derive or Merge for the
result's creation, while the source array still requires its one exact Merge.
Do not weaken a semantic purity guard to satisfy an incorrect topology guess.
