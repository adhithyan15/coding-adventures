// SIMPLE-level static Math.fround(n) fold -> numeric literal, ONLY at a
// float32 fixed point (constant-fold 0.110.0).
//
// Math.fround(x) (ECMAScript 21.3.2.19) rounds x to the nearest float32 and
// widens back to a double -- exactly `x as f32 as f64`, so the round-trip is
// bit-for-bit reproducible. The reference compiler folds ONLY when x is already
// an exact float32 (fround(x) === x), leaving the value unchanged:
//   * Math.fround(1.5)      -> 1.5        (1.5 is exactly a float32)
//   * Math.fround(-2.5)     -> -2.5
//   * Math.fround(0.25)     -> .25
//   * Math.fround(16777216) -> 16777216   (2^24 is exactly a float32)
//   * Math.fround(1.1)      -> DECLINED    (1.1 is not a float32; fround changes it)
//   * Math.fround(16777217) -> DECLINED    (2^24+1 rounds down to 2^24)
//   * m.fround(1.5)         -> DECLINED    (only the bare global Math folds)
//
// A -0 fixed point is also DECLINED (no numeric-literal spelling), and NaN /
// Infinity never reach the numeric-literal handler (they are identifiers, not
// numeric literals). Those are covered by unit tests, not shown here so the
// fixture stays byte-identical to the reference compiler.
var a = Math.fround(1.5);
var b = Math.fround(-2.5);
var c = Math.fround(0.25);
var d = Math.fround(16777216);
var e = Math.fround(1.1);
var f = Math.fround(16777217);
var g = m.fround(1.5);
report(a, b, c, d, e, f, g);
