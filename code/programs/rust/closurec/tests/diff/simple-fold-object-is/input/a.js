// SIMPLE-level static Object.is(a, b) fold → boolean (SameValue).
//
// Object.is (ECMAScript §20.1.2.13) compares with the SameValue algorithm,
// which differs from === in exactly two cases: Object.is(NaN, NaN) is true and
// Object.is(+0, -0) is false. We fold only when BOTH arguments are primitive
// literals whose values are known at compile time:
//   * Object.is(1, 1)     → true
//   * Object.is(0, -0)    → false  (the ±0 SameValue distinction; === gives true)
//   * Object.is("x", "x") → true
//   * Object.is(1, "1")   → false  (different Type)
//   * Object.is(NaN, NaN) → declined: `NaN` in source is the global *identifier*,
//                           not a numeric literal, so it is conservatively left
//                           alone (sound — the value of an identifier is unknown).
//
// Each value flows into report(...) so it stays referenced past
// remove-unused-vars and the fold is observable.
var a = Object.is(1, 1);
var b = Object.is(0, -0);
var c = Object.is("x", "x");
var d = Object.is(1, "1");
var e = Object.is(NaN, NaN);
report(a, b, c, d, e);
