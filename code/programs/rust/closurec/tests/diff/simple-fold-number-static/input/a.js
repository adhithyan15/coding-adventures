// SIMPLE-level static Number.isInteger / isFinite / isNaN fold → boolean.
//
// The ES2015 static predicates (ECMAScript §21.1.2.2/.3/.4) are STATIC METHODS
// on the global Number — modelled like String.fromCharCode. UNLIKE the global
// isNaN/isFinite, they do NO coercion: the argument must already be a Number,
// otherwise the answer is false. The fold collapses a call to a boolean literal
// when the single argument is a literal we can classify:
//   * Number.isInteger(42)   → true     a whole number
//   * Number.isInteger(3.5)  → false    has a fractional part
//   * Number.isInteger(1e21) → true     huge, but integer-valued
//   * Number.isFinite(42)    → true     finite
//   * Number.isNaN(42)       → false    a clean number is not NaN
//   * Number.isInteger("42") → false    a STRING is not a Number (no coercion!)
//   * Number.isFinite(null)  → false    null is not a Number
//
// Under WHITESPACE_ONLY every call survives; under SIMPLE they all collapse.
// Each value flows into report(...) so it stays referenced past
// remove-unused-vars and the fold is observable.
var a = Number.isInteger(42);
var b = Number.isInteger(3.5);
var c = Number.isInteger(1e21);
var d = Number.isFinite(42);
var e = Number.isNaN(42);
var f = Number.isInteger("42");
var g = Number.isFinite(null);
report(a, b, c, d, e, f, g);
