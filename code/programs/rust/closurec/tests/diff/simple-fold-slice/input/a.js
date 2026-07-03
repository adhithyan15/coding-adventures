// SIMPLE-level string-slicing fold (String#slice).
//
// `"<string>".slice(<start>[, <end>])` is compile-time-evaluable on a string
// literal with integer-literal arguments; the `constant-fold` pass collapses
// it to the substring (JS `String.prototype.slice`). `slice` indexes by
// UTF-16 code unit, with negative arguments counting from the end. Here
// `"abcd".slice(1, 3)` selects the half-open range `[1, 3)` → `"bc"`. Under
// WHITESPACE_ONLY the call survives; under SIMPLE it folds to `"bc"`.
//
// The value flows into `report(...)` so it stays referenced — otherwise
// remove-unused-vars (the last SIMPLE pass) would delete the declaration and
// the fold would not be observable.
var s = "abcd".slice(1, 3);
report(s);
