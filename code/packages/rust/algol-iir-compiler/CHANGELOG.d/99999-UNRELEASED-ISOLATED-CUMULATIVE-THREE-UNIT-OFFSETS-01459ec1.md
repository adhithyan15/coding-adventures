## Unreleased - Isolated cumulative three-unit offsets

Extend only the dedicated iterative direct-sign outer-entier proof to three
cumulative absolute units using canonical literals 0/1/2/3 and checked sums.
Explicit abs/entier wrappers do not reset budgets. Preserve the shared narrow
exp/log proof, legacy one-unit entry, one conditional front, exact read-once
identity lowering and alias/effect invalidation. Refuse multiple roots,
shadowed builtins, noncanonical integer/f64 folding traps and excess units.
Tests precede repair: focused 15/0, complete ALGOL 860/0, incoming runtime 1/0.
The six-package full run passed 1,779/0 with one existing ignore over 78 targets,
plus strict lint. Miri DEFERRED/PENDING, never passed. General composition and
full Closure parity/transformation lineage remain open.
