// SIMPLE-level static Array.of(...) fold → array literal [...].
//
// Array.of (ECMAScript §23.1.2.3) builds a fresh array whose elements are
// EXACTLY its arguments, in order. Unlike the Array(n) constructor — where a
// single numeric argument sets the LENGTH (Array(7) is a length-7 hole array) —
// Array.of(7) is the one-element array [7], so the fold is sound for ANY
// argument list (side effects of element expressions are preserved in order):
//   * Array.of()        → []
//   * Array.of(7)       → [7]        (NOT Array(7)'s length-7 array)
//   * Array.of(1, 2, 3) → [1, 2, 3]
//   * Array.of(x, y)    → [x, y]     (identifier args preserved)
//   * q.of(1)           → declined (only the bare global Array.of folds)
//
// Under WHITESPACE_ONLY every call survives; under SIMPLE the bare-global
// Array.of calls collapse to array literals. Each value flows into report(...)
// so it stays referenced past remove-unused-vars and the fold is observable.
var a = Array.of();
var b = Array.of(7);
var c = Array.of(1, 2, 3);
var d = Array.of(x, y);
var e = q.of(1);
report(a, b, c, d, e);
