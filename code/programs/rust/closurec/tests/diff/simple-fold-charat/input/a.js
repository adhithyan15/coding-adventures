// SIMPLE-level string-indexing fold (charCodeAt).
//
// `"<string>".charCodeAt(<int>)` is compile-time-evaluable on a string
// literal with an integer-literal index; the `constant-fold` pass collapses
// it to the UTF-16 code unit at that index (JS `String#charCodeAt`). Under
// WHITESPACE_ONLY the call survives; under SIMPLE it folds to `104` (the code
// for `h`).
//
// The value flows into `report(...)` so it stays referenced — otherwise
// remove-unused-vars (the last SIMPLE pass) would delete the declaration and
// the fold would not be observable.
var c = "hello".charCodeAt(0);
report(c);
