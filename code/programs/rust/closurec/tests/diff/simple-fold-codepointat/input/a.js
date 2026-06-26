// SIMPLE-level string-indexing fold (codePointAt).
//
// `"<string>".codePointAt(<int>)` is compile-time-evaluable on a string
// literal with a non-negative integer-literal index; the `constant-fold` pass
// collapses it to the Unicode code POINT starting at that UTF-16 code-unit
// index (JS `String#codePointAt`, ECMAScript §22.1.3.4). The defining
// difference from `charCodeAt`: an index landing on a high surrogate that is
// followed by a low surrogate combines the pair into one astral code point.
//
// Under WHITESPACE_ONLY the calls survive; under SIMPLE they fold:
//   * codePointAt(0) on "a💩b" → 97       (the BMP 'a', same as charCodeAt)
//   * codePointAt(1) on "a💩b" → 128169   (astral 💩 = U+1F4A9, the pair)
//   * codePointAt(1) on "💩"   → 56489    (the lone trailing low surrogate)
//
// Each value flows into `report(...)` so it stays referenced — otherwise
// remove-unused-vars (the last SIMPLE pass) would delete the declarations and
// the folds would not be observable.
var a = "a💩b".codePointAt(0);
var b = "a💩b".codePointAt(1);
var c = "💩".codePointAt(1);
report(a, b, c);
