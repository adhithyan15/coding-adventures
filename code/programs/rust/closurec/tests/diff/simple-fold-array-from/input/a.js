// SIMPLE-level static Array.from("...") fold → array of code-point strings.
//
// Array.from (ECMAScript §23.1.2.1) builds an array from an iterable. For a
// STRING the iterator yields one element per CODE POINT (like spread [..."..."]),
// so a string literal folds to an array literal of single-code-point strings:
//   * Array.from("abc") → ["a","b","c"]
//   * Array.from("")    → []
//   * Array.from("xy", f) → declined (a 2nd mapFn arg changes every element)
//   * Array.from(s)     → declined (a non-string-literal iterable is unknown)
//   * a.from("z")       → declined (only the bare global Array.from folds)
//
// (Astral/surrogate-pair handling — one element per astral char — is covered in
// the constant-fold crate's unit tests to keep this fixture's output ASCII.)
// Each value flows into report(...) so it stays referenced and the fold is observable.
var a = Array.from("abc");
var b = Array.from("");
var c = Array.from("xy", f);
var d = Array.from(s);
var e = q.from("z");
report(a, b, c, d, e);
