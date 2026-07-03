// SIMPLE-level static Array.isArray fold → boolean.
//
// The static Array.isArray(x) (ECMAScript §22.1.2.2) tests whether its argument
// is a real Array, with NO coercion. The fold collapses a call to a boolean
// literal for the literal shapes whose evaluation has no side effect to drop:
//   * Array.isArray([])    → true     (the only literal that IS an array)
//   * Array.isArray({})    → false    (an object is not an array)
//   * Array.isArray("x")   → false    (a string is not an array)
//   * Array.isArray(42)    → false
//   * Array.isArray(null)  → false
//   * Array.isArray([1,2]) → declined (folding would drop element evaluation)
//
// A NON-empty array/object literal is left intact (its elements might have side
// effects). Under WHITESPACE_ONLY every call survives; under SIMPLE the foldable
// ones collapse. Each value flows into report(...) so it stays referenced past
// remove-unused-vars and the fold is observable.
var a = Array.isArray([]);
var b = Array.isArray({});
var c = Array.isArray("x");
var d = Array.isArray(42);
var e = Array.isArray(null);
var f = Array.isArray([1, 2]);
report(a, b, c, d, e, f);
