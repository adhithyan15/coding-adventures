// SIMPLE-level global whole-URI fold (encodeURI / decodeURI).
//
// The global `encodeURI(str)` / `decodeURI(str)` functions (ECMAScript
// §19.2.6.4 / §19.2.6.2) are *free identifiers* — modelled like
// `encodeURIComponent`/`parseInt`. The constant-fold pass collapses a call to a
// string literal when the single argument is a string literal. They are the
// whole-URI siblings of the `…Component` encoders, differing only in how they
// treat the URI reserved/structural delimiters `; , / ? : @ & = + $` and `#`:
//   * encodeURI("a b")     → "a%20b"     (space → %20)
//   * encodeURI("a/b?c=d") → "a/b?c=d"   (reserved delimiters KEPT intact)
//   * encodeURI("é")       → "%C3%A9"    (each non-ASCII UTF-8 byte escaped)
//   * decodeURI("a%20b")   → "a b"       (the inverse)
//   * decodeURI("%2F")     → "%2F"       (a reserved escape stays ENCODED —
//                                         decodeURIComponent would give "/")
//   * decodeURI("%C3%A9")  → "é"
//
// `decodeURI` DECLINES (leaves the call) for a `URIError` input — a malformed
// escape or a byte run that is not valid UTF-8 — so a runtime throw is never
// folded into a value. `decodeURI("%E0")` (a truncated multi-byte scalar)
// therefore survives untouched.
//
// Under WHITESPACE_ONLY every call survives; under SIMPLE the foldable ones
// collapse. Each value flows into `report(...)` so it stays referenced past
// remove-unused-vars and the fold is observable.
var a = encodeURI("a b");
var b = encodeURI("a/b?c=d");
var c = encodeURI("é");
var d = decodeURI("a%20b");
var e = decodeURI("%2F");
var f = decodeURI("%C3%A9");
var g = decodeURI("%E0");
report(a, b, c, d, e, f, g);
