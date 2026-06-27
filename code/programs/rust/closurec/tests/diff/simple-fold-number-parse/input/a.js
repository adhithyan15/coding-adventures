// SIMPLE-level static Number.parseInt / Number.parseFloat fold → numeric.
//
// The ES2015 static methods (ECMAScript §21.1.2.12/.13) are the SAME function
// objects as the global parseInt/parseFloat (Number.parseInt === parseInt), so
// they run the identical leading-prefix scan. The fold collapses a call to a
// numeric literal when the single argument is a string literal (parseInt takes
// an optional integer-literal radix):
//   * Number.parseInt("12px")   → 12     (trailing garbage ignored)
//   * Number.parseInt("FF", 16) → 255    (explicit radix)
//   * Number.parseInt("0x1F")   → 31     (0x prefix → hex)
//   * Number.parseFloat("3.14e2abc") → 314  (leading float prefix)
//   * Number.parseInt("")       → declined (NaN has no literal token)
//
// `Number.parseInt("")` is NaN — no numeric literal — so that call survives.
// Under WHITESPACE_ONLY every call survives; under SIMPLE the foldable ones
// collapse. Each value flows into report(...) so it stays referenced past
// remove-unused-vars and the fold is observable.
var a = Number.parseInt("12px");
var b = Number.parseInt("FF", 16);
var c = Number.parseInt("0x1F");
var d = Number.parseFloat("3.14e2abc");
var e = Number.parseInt("");
report(a, b, c, d, e);
