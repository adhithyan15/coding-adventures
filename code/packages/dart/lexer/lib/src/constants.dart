/// Constants shared by the Dart lexer implementations.
///
/// Standing Rule #20: Magic numbers live here, not inline in operational code.
library;

/// Largest Unicode code point representable as a single UTF-16 code unit.
const int maxBmpCodePoint = 0xffff;

/// UTF-16 code units occupied by a code point at or below [maxBmpCodePoint].
const int utf16UnitsPerBmpCodePoint = 1;

/// UTF-16 code units occupied by an astral code point (a surrogate pair).
const int utf16UnitsPerAstralCodePoint = 2;

/// Code point of the line feed character that starts a new source line.
const int lineFeedCodePoint = 0x0a;

/// ASCII code point for a horizontal tab character.
const int tabCodePoint = 0x09;

/// ASCII code point for a carriage return character.
const int carriageReturnCodePoint = 0x0d;

/// ASCII code point for a space character.
const int spaceCodePoint = 0x20;
