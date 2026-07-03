// SIMPLE-level string-substring fold (String#substring).
//
// `"<string>".substring(<start>[, <end>])` is compile-time-evaluable on a
// string literal with integer-literal arguments; the `constant-fold` pass
// collapses it to the substring (JS `String.prototype.substring`). Unlike
// `slice`, `substring` clamps each index into `[0, len]` (a negative argument
// becomes 0 — it never counts from the end) and SWAPS the endpoints when
// `start > end`. The four cases below exercise exactly those rules:
//
//   "abcd".substring(1, 3) → "bc"    (the plain half-open range [1, 3))
//   "abcd".substring(3, 1) → "bc"    (start > end → endpoints swap)
//   "abcd".substring(-2)   → "abcd"  (a negative start clamps to 0)
//   "abcd".substring(10)   → ""      (a start past the end clamps to len)
//
// Under WHITESPACE_ONLY the calls survive; under SIMPLE they all fold.
//
// The values flow into `report(...)` so they stay referenced — otherwise
// remove-unused-vars (the last SIMPLE pass) would delete the declarations and
// the folds would not be observable.
var a = "abcd".substring(1, 3);
var b = "abcd".substring(3, 1);
var c = "abcd".substring(-2);
var d = "abcd".substring(10);
report(a, b, c, d);
