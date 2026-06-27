// SIMPLE-level static Object.keys/values/entries({}) fold → [].
//
// Object.keys/values/entries(x) (ECMAScript §20.1.2) enumerate an object's own
// enumerable keys. For an EMPTY object literal {} the result is ALWAYS the empty
// array [] — no keys, and evaluating {} has no side effect — so the fold
// collapses the call to []:
//   * Object.keys({})    → []
//   * Object.values({})  → []
//   * Object.entries({}) → []
//   * Object.keys({a:1}) → declined (property values may have side effects)
//   * Object.keys([])    → declined (an array's keys are its indices)
//
// A non-empty object, an array, a primitive, or a non-literal is left intact.
// Under WHITESPACE_ONLY every call survives; under SIMPLE the foldable ones
// collapse. Each value flows into report(...) so it stays referenced past
// remove-unused-vars and the fold is observable.
var a = Object.keys({});
var b = Object.values({});
var c = Object.entries({});
var d = Object.keys({a: 1});
var e = Object.keys([]);
report(a, b, c, d, e);
