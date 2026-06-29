// SIMPLE-level static Math.max(...) / Math.min(...) fold → numeric literal.
//
// Math.max / Math.min (ECMAScript §21.3.2.24/.25) return the largest / smallest
// argument after ToNumber. When every argument is a numeric literal the result
// is known at compile time:
//   * Math.max(1, 2, 3) → 3      Math.min(1, 2, 3) → 1
//   * Math.max(-5, -1)  → -1     Math.min(-5, -1)  → -5
//   * Math.max(7)       → 7      (single argument)
//   * Math.max(1, x)    → declined (x is a non-literal, runtime-unknown)
//   * Math.max()        → declined (would be -Infinity)
//   * m.max(1, 2)       → declined (only the bare global Math)
//
// Under WHITESPACE_ONLY every call survives; under SIMPLE the all-numeric
// bare-global calls collapse to a literal. Each value flows into report(...).
var a = Math.max(1, 2, 3);
var b = Math.min(1, 2, 3);
var c = Math.max(-5, -1);
var d = Math.max(7);
var e = Math.max(1, x);
var f = Math.max();
var g = m.max(1, 2);
report(a, b, c, d, e, f, g);
