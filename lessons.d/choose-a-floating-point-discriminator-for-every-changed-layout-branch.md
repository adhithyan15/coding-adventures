---
category: Testing & coverage
---

# Choose a floating-point discriminator for every changed layout branch

A proposed four-event Timeline test distinguished cumulative addition from
indexed multiplication in TD, but its default LR origin happened to produce
identical event coordinates under both formulas. An LR row-span witness alone
could miss a partial repair that retained indexed event positions. Independent
preflight caught this before execution. Retain the original complete products
and add a complete LR product with padding0: fractional size16.01 then changes
the second and third cumulative advances by represented floating-point bits.
Verify each changed branch has its own distinguishing input before treating a
shared numerical fixture as coverage of every branch.
