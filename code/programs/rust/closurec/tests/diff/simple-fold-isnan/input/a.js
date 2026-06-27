// SIMPLE-level global isNaN / isFinite fold → boolean.
//
// The global predicates `isNaN(x)` / `isFinite(x)` (ECMAScript §19.2.3 /
// §19.2.2) are *free identifiers* — modelled like `Number`/`Boolean`. Each
// coerces its argument with ToNumber and tests the result: isNaN is true iff
// the number is NaN; isFinite is true iff it is neither NaN nor +/-Infinity.
// The constant-fold pass collapses a call to a boolean literal when the single
// argument is a string- or number-literal (booleans print as !0 / !1):
//   * isNaN("abc")       → true   (!0)   "abc" coerces to NaN
//   * isNaN("42")        → false  (!1)   a clean number
//   * isNaN(" ")         → false  (!1)   ToNumber(" ") is +0, not NaN
//   * isFinite("1e3")    → true   (!0)   1000 is finite
//   * isFinite("Infinity") → false (!1)  +Infinity is not finite
//   * isFinite("abc")    → false  (!1)   NaN is not finite
//   * isFinite(0)        → true   (!0)   a finite number literal
//
// Unlike Number(...), no shape declines: every string has a well-defined
// NaN/Infinity/finite class. Under WHITESPACE_ONLY every call survives; under
// SIMPLE they all collapse. Each value flows into report(...) so it stays
// referenced past remove-unused-vars and the fold is observable.
var a = isNaN("abc");
var b = isNaN("42");
var c = isNaN(" ");
var d = isFinite("1e3");
var e = isFinite("Infinity");
var f = isFinite("abc");
var g = isFinite(0);
report(a, b, c, d, e, f, g);
