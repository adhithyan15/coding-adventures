// SIMPLE-level global URI-component fold (encodeURIComponent / decodeURIComponent).
//
// The global `encodeURIComponent(str)` / `decodeURIComponent(str)` functions
// (ECMAScript §19.2.6.5 / §19.2.6.3) are *free identifiers* — modelled like
// `parseInt`/`parseFloat`. The constant-fold pass collapses a call to a string
// literal when the single argument is a string literal:
//   * encodeURIComponent("a b")  → "a%20b"   (space → %20)
//   * encodeURIComponent("é")    → "%C3%A9"   (each UTF-8 byte percent-escaped)
//   * encodeURIComponent("/")    → "%2F"      (reserved delimiters ARE escaped)
//   * decodeURIComponent("a%20b")→ "a b"      (the inverse)
//   * decodeURIComponent("%C3%A9")→ "é"
//
// `decodeURIComponent` DECLINES (leaves the call) for a `URIError` input — a
// malformed escape or a byte run that is not valid UTF-8 — so a runtime throw
// is never folded into a value. `decodeURIComponent("%E0")` (a truncated
// multi-byte scalar) therefore survives untouched.
//
// Under WHITESPACE_ONLY every call survives; under SIMPLE the foldable ones
// collapse. Each value flows into `report(...)` so it stays referenced past
// remove-unused-vars and the fold is observable.
var a = encodeURIComponent("a b");
var b = encodeURIComponent("é");
var c = encodeURIComponent("/");
var d = decodeURIComponent("a%20b");
var e = decodeURIComponent("%C3%A9");
var f = decodeURIComponent("%E0");
report(a, b, c, d, e, f);
