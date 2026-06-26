// SIMPLE-level string last-occurrence search fold (String#lastIndexOf).
//
// `"<haystack>".lastIndexOf("<needle>")` is compile-time-evaluable when both
// are string literals; the `constant-fold` pass collapses it to the UTF-16
// code-unit index of the LAST occurrence, or -1 when absent (ECMAScript
// §22.1.3.9, the one-argument form). It is the mirror of the already-folded
// `indexOf` (which finds the FIRST occurrence). The four cases below cover
// last-match, absent, empty-needle (→ string length), and astral indexing:
//
//   "abcabc".lastIndexOf("bc") → 4   (the last "bc")
//   "abcabc".lastIndexOf("z")  → -1  (absent)
//   "abc".lastIndexOf("")      → 3   (empty needle → string length)
//   "ab".lastIndexOf("b")      → 1
//
// Under WHITESPACE_ONLY the calls survive; under SIMPLE they all fold.
//
// The values flow into `report(...)` so they stay referenced — otherwise
// remove-unused-vars (the last SIMPLE pass) would delete the declarations and
// the folds would not be observable.
var a = "abcabc".lastIndexOf("bc");
var b = "abcabc".lastIndexOf("z");
var c = "abc".lastIndexOf("");
var d = "ab".lastIndexOf("b");
report(a, b, c, d);
