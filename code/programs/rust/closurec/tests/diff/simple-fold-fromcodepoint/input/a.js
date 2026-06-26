// SIMPLE-level static-method fold (String.fromCodePoint).
//
// `String.fromCodePoint(cp0, cp1, …)` builds a string from Unicode CODE POINTS
// (ECMAScript §22.1.2.2). Unlike fromCharCode (UTF-16 units), each argument is
// a whole code point, so one astral argument suffices. The constant-fold pass
// collapses it to a string literal when every argument is a non-negative
// integer literal that is a valid Unicode scalar (0..=0x10FFFF, not a surrogate).
//
// Under WHITESPACE_ONLY the calls survive; under SIMPLE they fold:
//   * fromCodePoint(72, 73)      → "HI"   (two BMP scalars)
//   * fromCodePoint(128169)      → "💩"   (a SINGLE astral arg = U+1F4A9)
//   * fromCodePoint(128169, 65)  → "💩A"
//
// Each value flows into `report(...)` so it stays referenced past the
// remove-unused-vars pass and the fold is observable.
var a = String.fromCodePoint(72, 73);
var b = String.fromCodePoint(128169);
var c = String.fromCodePoint(128169, 65);
report(a, b, c);
