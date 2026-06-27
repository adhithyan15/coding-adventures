// SIMPLE-level legacy global escaper fold (escape / unescape).
//
// The legacy `escape(str)` / `unescape(str)` globals (ECMAScript Annex B
// §B.2.1.1 / §B.2.1.2) are *free identifiers* — modelled like `encodeURI`. The
// constant-fold pass collapses a call to a string literal when the single
// argument is a string literal. UNLIKE the `…URI` encoders, they operate on
// UTF-16 CODE UNITS, not UTF-8 bytes:
//   * escape("a b")   → "a%20b"        (space → %20)
//   * escape("~/@")   → "%7E/@"        (~ escaped; / and @ are unescaped marks)
//   * escape("é")     → "%E9"          (U+00E9 is ONE code unit < 0x100 → %XX)
//   * escape("😀")    → "%uD83D%uDE00" (one astral scalar → two surrogate units)
//   * unescape("a%20b") → "a b"        (the inverse)
//   * unescape("%E9")   → "é"          (emitted as é)
//   * unescape("%2F")   → "/"          (EVERY escape decodes, unlike decodeURI)
//
// `unescape` DECLINES (leaves the call) when its result would contain an
// unpaired surrogate — `unescape("%uD83D")` is a lone high surrogate, which has
// no string-literal representation — so that call survives untouched.
//
// Under WHITESPACE_ONLY every call survives; under SIMPLE the foldable ones
// collapse. Each value flows into `report(...)` so it stays referenced past
// remove-unused-vars and the fold is observable.
var a = escape("a b");
var b = escape("~/@");
var c = escape("é");
var d = escape("😀");
var e = unescape("a%20b");
var f = unescape("%E9");
var g = unescape("%2F");
var h = unescape("%uD83D");
report(a, b, c, d, e, f, g, h);
