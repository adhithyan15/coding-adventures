## 0.266.0 — 2026-09-12 — bounded while-loop scalar recurrences

Statically bounded `while` elements now simulate one simple local numeric
assignment after each true predicate, so exact integer and finite real
recurrences retain their final snapshot. The existing 4,096-pass cap and
control-dependency checks reject nontermination, overflow, non-finite values,
dynamic predicates, dependency writes, compound bodies, and nonnumeric targets.

