// Concise-body arrow functions in common positions — closurec must optimise
// INSIDE each arrow body (constant-fold), not fall back to WHITESPACE_ONLY.
// (Block-bodied arrows `x => { ... }` are blocked on a grammar limitation —
// see CLOC12-gaps gap-156 — and object-body arrows decline to avoid the
// `() => {}` empty-block-vs-object ambiguity.)
var factory = n => 1 + 2;          // concise body: 1+2 folds to 3
var scaled = (a, b) => a + 3 * 4;  // multi-param: 3*4 folds to 12 inside
arr.map(x => x + (5 - 1));         // callback argument: 5-1 folds to 4
