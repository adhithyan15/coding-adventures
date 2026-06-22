// SIMPLE-level string-repeat fold (String#repeat).
//
// `"<string>".repeat(<count>)` is compile-time-evaluable on a string literal
// with a non-negative integer-literal count; the `constant-fold` pass collapses
// it to the receiver concatenated `count` times (JS `String.prototype.repeat`).
// Here `"ab".repeat(3)` → `"ababab"`. A negative count (JS `RangeError`), a
// fractional count, or a result over the optimizer's size cap is left for the
// runtime. Under WHITESPACE_ONLY the call survives; under SIMPLE it folds.
//
// The value flows into `report(...)` so it stays referenced — otherwise
// remove-unused-vars (the last SIMPLE pass) would delete the declaration and
// the fold would not be observable.
var s = "ab".repeat(3);
report(s);
