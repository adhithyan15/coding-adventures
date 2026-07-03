// SIMPLE-level control-flow folding (CLOC12.156).
//
// The SIMPLE pipeline now runs `fold-control-flow` after `constant-fold`.
// Each `if` below has a statically-decidable condition, so the dead branch
// is pruned:
//
//   - `if (2 > 3)`  — constant-fold first turns `2 > 3` into `false`, THEN
//     fold-control-flow keeps the `else` branch. This proves the two passes
//     compose (the comparison must be folded before the branch can be).
//   - `if (true)`   — keeps the consequent.
//   - `if (4 > 5)`  — folds to `false` with no `else`, so the whole `if`
//     becomes an empty statement.
//
// Under WHITESPACE_ONLY none of this happens — every `if` survives verbatim.
if (2 > 3) { keepElse(); } else { takeThis(); }
if (true) { alsoKept(); } else { dropped(); }
if (4 > 5) { vanishes(); }
