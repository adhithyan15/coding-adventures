# Fixture: `simple-fold-fromcharcode`

End-to-end oracle for the static `String.fromCharCode(...)` fold at
`--compilation_level SIMPLE`.

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | three `String.fromCharCode(...)` calls into `report(...)` |
| `expected.stdout` | `var a="HI";var b="💩";var c="";report(a,b,c);` |

The SIMPLE level runs the typed-AST optimization pipeline, whose
`constant-fold` pass evaluates `String.fromCharCode` (ECMAScript §22.1.2.1) when
every argument is a non-negative integer literal in `0..=0xFFFF`. The arguments
are UTF-16 code **units**:

| Call | Result | Why |
|------|--------|-----|
| `String.fromCharCode(72, 73)` | `"HI"` | two BMP units `H`,`I` |
| `String.fromCharCode(0xD83D, 0xDCA9)` | `"💩"` | a high+low surrogate PAIR → `U+1F4A9` |
| `String.fromCharCode()` | `""` | no arguments |

This is the first fold whose receiver is the bare global `String` (not a
string/number literal). The emitter prints the astral scalar as its escaped
UTF-16 surrogate pair `💩` — byte-for-byte equal to `"💩"`. Out-of-
range / fractional / lone-surrogate arguments are left unfolded. Under
`WHITESPACE_ONLY` the calls survive untouched; the values flow into `report(...)`
so they stay referenced past remove-unused-vars and the fold is observable.
