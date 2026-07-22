// SIMPLE-level static Math.trunc(n) / Math.sign(n) fold -> numeric literal.
//
// Math.trunc (ECMAScript 21.3.2.38) removes the fractional part, rounding
// toward zero; Math.sign (21.3.2.34) yields -1 / -0 / +0 / +1 (and NaN for NaN).
// When the single argument is a numeric literal the result is known at compile
// time and the call collapses to a literal:
//   * Math.trunc(4.9)  -> 4       Math.trunc(-4.9) -> -4
//   * Math.sign(7)     -> 1       Math.sign(-3)    -> -1     Math.sign(0) -> 0
//   * Math.trunc(x)    -> declined (x is a non-literal, runtime-unknown)
//   * Math.sqrt(16)    -> declined (the reference compiler does NOT fold the
//                         transcendental Math methods, even when exact)
//   * m.trunc(1.5)     -> declined (only the bare global Math folds)
//
// A -0 result (e.g. Math.trunc(-0.5) === -0) is also DECLINED, matching how the
// same handler already declines Math.ceil(-0.5): -0 has no faithful numeric-
// literal spelling. (Covered by unit tests, not shown here so this fixture
// stays byte-identical to the reference compiler.)
//
// Under WHITESPACE_ONLY every call survives; under SIMPLE the bare-global
// numeric-literal calls collapse. Each value flows into report(...).
var a = Math.trunc(4.9);
var b = Math.trunc(-4.9);
var c = Math.sign(7);
var d = Math.sign(-3);
var e = Math.sign(0);
var f = Math.trunc(x);
var g = Math.sqrt(16);
var h = m.trunc(1.5);
report(a, b, c, d, e, f, g, h);
