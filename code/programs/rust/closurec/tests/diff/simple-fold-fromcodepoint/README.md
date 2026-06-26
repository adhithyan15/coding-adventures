# Fixture: `simple-fold-fromcodepoint`

End-to-end oracle for the static `String.fromCodePoint(...)` fold at
`--compilation_level SIMPLE`.

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | three `String.fromCodePoint(...)` calls into `report(...)` |
| `expected.stdout` | `var a="HI";var b="💩";var c="💩A";report(a,b,c);` |

The SIMPLE level runs the typed-AST optimization pipeline, whose
`constant-fold` pass evaluates `String.fromCodePoint` (ECMAScript §22.1.2.2)
when every argument is a non-negative integer literal that is a valid Unicode
scalar (`0..=0x10FFFF`, not a surrogate). Each argument is a whole **code
point**, the defining difference from `fromCharCode` (UTF-16 *units*):

| Call | Result | Why |
|------|--------|-----|
| `String.fromCodePoint(72, 73)` | `"HI"` | two BMP scalars |
| `String.fromCodePoint(128169)` | `"💩"` | a SINGLE astral arg = `U+1F4A9` |
| `String.fromCodePoint(128169, 65)` | `"💩A"` | astral + `A` |

The emitter prints the astral scalar as its escaped UTF-16 surrogate pair
(`💩` → `💩`), byte-for-byte equal to `"💩"`. A surrogate code point,
an out-of-range (`>0x10FFFF`) / negative / fractional argument (each a JS
`RangeError`), or a non-literal argument is left unfolded. Under
`WHITESPACE_ONLY` the calls survive untouched; the values flow into `report(...)`
so they stay referenced past remove-unused-vars and the fold is observable.
