---
category: Testing & coverage
---

# Scope validator substitutions to the exact census being changed

The typography runner's v2 derivation changed range(8)} globally to range(9)}
to admit nine new empty typography cases. It also changed the unrelated,
retained Timeline-size priority census from eight controls to a phantom ninth
control. Those tests still correctly declare and emit 104 complete markers.
Use an exact, uniquely anchored substitution for the intended census and
inspect the entire helper diff before running it.

The active source and runner remained frozen after discovering the mistake.
Preserve the original complete execution receipt and its actual outer outcome.
A separate read-only adjudicator may accept the real command results only
after checking every raw stream, status, artifact identity and frozen input,
and independently resolving precisely the incorrect marker expectation. Never
relabel an original wrapper failure as a successful execution or reduce any
production/test limit to satisfy a mistaken validator.
