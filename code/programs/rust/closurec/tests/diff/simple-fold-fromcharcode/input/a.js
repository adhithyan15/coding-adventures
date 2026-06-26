// SIMPLE-level static-method fold (String.fromCharCode).
//
// `String.fromCharCode(u0, u1, …)` builds a string from UTF-16 code UNITS
// (ECMAScript §22.1.2.1). The constant-fold pass collapses it to a string
// literal when every argument is a non-negative integer literal in 0..=0xFFFF.
// This is the first fold whose receiver is the bare global `String` rather
// than a string/number literal.
//
// Under WHITESPACE_ONLY the calls survive; under SIMPLE they fold:
//   * fromCharCode(72, 73)       → "HI"   (two BMP units)
//   * fromCharCode(0xD83D,0xDCA9)→ "💩"   (a high+low surrogate PAIR → U+1F4A9)
//   * fromCharCode()             → ""     (no arguments)
//
// Each value flows into `report(...)` so it stays referenced past the
// remove-unused-vars pass and the fold is observable.
var a = String.fromCharCode(72, 73);
var b = String.fromCharCode(0xD83D, 0xDCA9);
var c = String.fromCharCode();
report(a, b, c);
