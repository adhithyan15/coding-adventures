// SIMPLE-level static Object.entries({k: v, …}) fold → array of [key, value]
// pairs. Object.entries (ECMAScript §20.1.2.5) is the inverse of
// Object.fromEntries: it lists an object's own enumerable string-keyed entries.
// For a fully-static object literal of plain data properties with primitive
// literal values, the result is known at compile time:
//   * Object.entries({a: 1, b: 2}) → [["a",1],["b",2]]
//   * Object.entries({x: "hi"})    → [["x","hi"]]
//   * Object.entries({})           → []           (empty case)
//   * Object.entries({1: "x"})     → declined (integer-index key reorders)
//   * Object.entries({__proto__:1}) → declined (prototype setter, not own prop)
//   * o.entries({a: 1})            → declined (only the bare global Object)
//
// Under WHITESPACE_ONLY every call survives; under SIMPLE the foldable
// bare-global Object.entries calls collapse. Each value flows into report(...)
// so it stays referenced past remove-unused-vars.
var a = Object.entries({a: 1, b: 2});
var b = Object.entries({x: "hi"});
var c = Object.entries({});
var d = Object.entries({1: "x"});
var e = Object.entries({__proto__: 1});
var f = o.entries({a: 1});
report(a, b, c, d, e, f);
