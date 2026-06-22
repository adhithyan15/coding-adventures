// SIMPLE-level string-literal `.length` folding.
//
// `"<string>".length` is a compile-time-evaluable expression that the
// `constant-fold` pass collapses to the UTF-16 code-unit count (JS
// `String#length` semantics). Under WHITESPACE_ONLY it survives as
// `"hello".length`; under SIMPLE it folds to `5`.
//
// The value flows into `report(...)` so it stays referenced — otherwise
// remove-unused-vars (the last SIMPLE pass) would delete the declaration
// and the fold would not be observable.
var n = "hello".length;
report(n);
