// ADVANCED-level optimization (CLOC12.161).
//
// ADVANCED used to be a literal no-op (it returned the source verbatim).
// It now runs the same typed optimization pipeline as SIMPLE — it is
// specified to be at least as aggressive — so this input is folded,
// dead-code-eliminated, and renamed exactly as SIMPLE would do it:
//
//   - `1 + 2` folds to `3`;
//   - the unused `var dead` is removed;
//   - `compute`'s parameter `longName` is shortened to `a`.
//
// The value use `sink(compute)` (passing it without calling) makes the
// inliner decline `compute`, keeping this fixture's demonstration on
// fold + dead-code removal + rename rather than inlining (which has its
// own pass-crate tests).
//
// Advanced-only passes (aggressive property/global renaming, cross-module
// tree-shaking) layer on as they are implemented.
var dead = 1 + 2;
function compute(longName) {
  return longName + 1;
}
report(compute(7));
sink(compute);
