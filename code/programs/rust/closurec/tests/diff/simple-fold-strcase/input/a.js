// SIMPLE-level ASCII string-casing fold.
//
// `"<ascii>".toUpperCase()` / `.toLowerCase()` are compile-time-evaluable on
// a string literal; the `constant-fold` pass collapses them (ASCII case
// mapping is locale-independent and matches JS exactly). Under WHITESPACE_ONLY
// the call survives as `"hello".toUpperCase()`; under SIMPLE it folds to
// `"HELLO"`.
//
// The value flows into `report(...)` so it stays referenced — otherwise
// remove-unused-vars (the last SIMPLE pass) would delete the declaration and
// the fold would not be observable.
var s = "hello".toUpperCase();
report(s);
