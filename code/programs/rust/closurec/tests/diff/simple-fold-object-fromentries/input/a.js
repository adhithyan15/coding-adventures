// SIMPLE-level static Object.fromEntries([[k, v], …]) fold → object literal.
//
// Object.fromEntries (ECMAScript §20.1.2.7) is the inverse of Object.entries:
// it walks an array of [key, value] pairs and builds a plain object. We fold
// the fully-static shape — a single array literal of 2-element [key, value]
// array literals, each key a string/numeric literal and each value a primitive
// literal:
//   * Object.fromEntries([["a", 1], ["b", 2]]) → {a: 1, b: 2}
//   * Object.fromEntries([[1, "x"]])           → {"1": "x"}  (numeric key ToString)
//   * Object.fromEntries([["a", 1], ["a", 2]]) → {a: 2}      (duplicate: last wins)
//   * Object.fromEntries([])                   → {}
//   * o.fromEntries([["a", 1]])                → declined (only bare global Object)
//   * Object.fromEntries([["__proto__", 1]])  → declined (own-prop vs proto setter)
//
// Under WHITESPACE_ONLY every call survives; under SIMPLE the bare-global
// Object.fromEntries calls collapse to object literals. Each value flows into
// report(...) so it stays referenced past remove-unused-vars.
var a = Object.fromEntries([["a", 1], ["b", 2]]);
var b = Object.fromEntries([[1, "x"]]);
var c = Object.fromEntries([["a", 1], ["a", 2]]);
var d = Object.fromEntries([]);
var e = o.fromEntries([["a", 1]]);
var f = Object.fromEntries([["__proto__", 1]]);
report(a, b, c, d, e, f);
