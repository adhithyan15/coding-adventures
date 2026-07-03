// SIMPLE-level static Object.keys/values/entries fold.
//
// Object.keys/values/entries(x) (ECMAScript §20.1.2) enumerate an object's own
// enumerable keys. Two folds apply here:
//   * EMPTY object literal {} → [] for all three methods (no keys, evaluating {}
//     has no side effect): Object.keys/values/entries({}) → [].
//   * NON-EMPTY object literal with static data properties → array of its own
//     string keys, for Object.keys: Object.keys({a:1,b:2}) → ["a","b"].
//
// Declined (left intact): Object.values of a non-empty object (no non-empty fold
// yet), Object.keys of an integer-index-keyed object (indices enumerate first,
// reordering the result), and Object.keys of an array (an array's keys are its
// indices). A primitive or non-literal is also declined.
//   * Object.keys({})        → []
//   * Object.values({})      → []
//   * Object.entries({})     → []
//   * Object.keys({a:1,b:2}) → ["a","b"]
//   * Object.values({a:1})   → declined (no non-empty values fold)
//   * Object.keys({1:"x"})   → declined (integer-index key reorders)
//   * Object.keys([])        → declined (an array's keys are its indices)
//
// Under WHITESPACE_ONLY every call survives; under SIMPLE the foldable ones
// collapse. Each value flows into report(...) so it stays referenced past
// remove-unused-vars and the fold is observable.
var a = Object.keys({});
var b = Object.values({});
var c = Object.entries({});
var d = Object.keys({a: 1, b: 2});
var e = Object.values({a: 1});
var f = Object.keys({1: "x"});
var g = Object.keys([]);
report(a, b, c, d, e, f, g);
