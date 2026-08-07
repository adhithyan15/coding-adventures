# Fixture: `simple-fold-codepointat`

End-to-end oracle for string-indexing folding (`codePointAt`) at
`--compilation_level SIMPLE`.

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | three `"…".codePointAt(i)` calls into `report(...)` |
| `expected.stdout` | The folded output: `var a=97,b=128169,c=56489;report(a,b,c);` |

The SIMPLE level runs the typed-AST optimization pipeline, whose
`constant-fold` pass evaluates `String#codePointAt` on a string-literal
receiver with a non-negative integer-literal index (ECMAScript §22.1.3.4).

`codePointAt` indexes by UTF-16 code **unit** but returns a code **point**:
when the unit at `i` is a high surrogate followed by a low surrogate, the pair
is combined into one astral code point. That is the defining difference from
`charCodeAt`, which returns the bare 16-bit unit.

| Call | UTF-16 units | Result | Why |
|------|--------------|--------|-----|
| `"a💩b".codePointAt(0)` | `[0x61, 0xD83D, 0xDCA9, 0x62]` | `97` | BMP `a`; same as `charCodeAt(0)` |
| `"a💩b".codePointAt(1)` | (as above) | `128169` | high+low surrogate → `U+1F4A9` 💩 |
| `"💩".codePointAt(1)`   | `[0xD83D, 0xDCA9]` | `56489` | lone trailing low surrogate `0xDCA9` |

An out-of-range index is JS `undefined` (no literal) and is left unfolded.
Under `WHITESPACE_ONLY` the calls survive untouched; the fold is a SIMPLE/
ADVANCED typed-pipeline transform. The values flow into `report(...)` so they
stay referenced past the remove-unused-vars pass and the fold is observable.
