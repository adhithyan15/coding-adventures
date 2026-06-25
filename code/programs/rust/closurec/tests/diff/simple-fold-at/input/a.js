// SIMPLE-level string-index fold (String#at, negative-from-end).
//
// `"<string>".at(<index>)` is compile-time-evaluable on a string literal with
// an integer-literal index; the `constant-fold` pass collapses it to the
// one-code-unit string at that index (JS `String.prototype.at`). Unlike
// `charAt`, a NEGATIVE index counts from the end — here `"abcde".at(-2)` is the
// second-to-last character, `"d"`. An out-of-range index (JS `undefined`, no
// literal), a fractional/non-literal index, or a lone-surrogate result is left
// for the runtime. Under WHITESPACE_ONLY the call survives; under SIMPLE it
// folds.
//
// The value flows into `report(...)` so it stays referenced — otherwise
// remove-unused-vars (the last SIMPLE pass) would delete the declaration and
// the fold would not be observable.
var s = "abcde".at(-2);
report(s);
