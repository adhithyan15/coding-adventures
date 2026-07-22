// SIMPLE-level static Math.clz32(n) / Math.imul(a,b) fold -> numeric literal.
//
// clz32 counts the leading zero bits of ToUint32(n) (0..32); imul is the 32-bit
// signed product of ToUint32(a) and ToUint32(b). Both are pure modular integer
// operations, so the fold is bit-exact and byte-identical to the reference:
//   * Math.clz32(1)      -> 31     Math.clz32(0)     -> 32
//   * Math.clz32(-1)     -> 0      (ToUint32(-1) = 2**32-1, top bit set)
//   * Math.imul(3, 4)    -> 12     Math.imul(-1, 5)  -> -5
//   * Math.imul(65536, 65536) -> 0 (product 2**32, low 32 bits are 0)
//   * Math.clz32(x)      -> declined (x is a non-literal, runtime-unknown)
//   * m.imul(2, 3)       -> declined (only the bare global Math)
//
// Under WHITESPACE_ONLY every call survives; under SIMPLE the bare-global
// numeric-literal calls collapse. Each value flows into report(...).
var a = Math.clz32(1);
var b = Math.clz32(0);
var c = Math.clz32(-1);
var d = Math.imul(3, 4);
var e = Math.imul(-1, 5);
var f = Math.imul(65536, 65536);
var g = Math.clz32(x);
var h = m.imul(2, 3);
report(a, b, c, d, e, f, g, h);
