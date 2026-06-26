// SIMPLE-level legacy string-substr fold (String#substr).
//
// `"<string>".substr(<start>[, <length>])` is compile-time-evaluable on a
// string literal with integer-literal arguments; the `constant-fold` pass
// collapses it (JS `String.prototype.substr`, ECMAScript Annex B §B.2.3.1).
// Unlike `slice`/`substring`, substr's SECOND argument is a *length*, not an
// end index. A negative start counts from the end (then clamps to 0); the
// length clamps into [0, len - start]. The four cases below exercise exactly
// those rules:
//
//   "abcde".substr(1, 2) → "bc"    (start 1, take 2)
//   "abcde".substr(1)    → "bcde"  (length defaults to the rest)
//   "abcde".substr(-2)   → "de"    (negative start counts from the end)
//   "abcde".substr(10)   → ""      (start past the end → empty)
//
// Under WHITESPACE_ONLY the calls survive; under SIMPLE they all fold.
//
// The values flow into `report(...)` so they stay referenced — otherwise
// remove-unused-vars (the last SIMPLE pass) would delete the declarations and
// the folds would not be observable.
var a = "abcde".substr(1, 2);
var b = "abcde".substr(1);
var c = "abcde".substr(-2);
var d = "abcde".substr(10);
report(a, b, c, d);
