### KILN01: specify the Kiln language

- New spec `code/specs/KILN01-language.md`. KILN00 §6 says "KILN01 specifies the
  full grammar" and KILN02 reserves `SemanticsVersion == 1` for exactly this
  expansion, but the file did not exist — so there was nothing for a parser, a
  linter or a conformance port to agree on, and M4's rule files had no
  definition to implement against.
- It specifies, and anchors every construct to KILN00 rather than inventing:
  the two file kinds and their markers; the lexical layer (UTF-8 rules,
  spaces-only consistent indentation, the token set, `$` interpolation); the
  statement, field, append, phase and step grammar; variables and `for`
  comprehensions with the termination rule; expansion to the canonical action
  and its canonical form; the diagnostic tokens; and the two pure functions of
  the KILN00 §16 conformance surface with the vectors they must reproduce.
- It states what a value may NOT do (read a file, clock, environment, network
  or a glob at evaluation time), because that is what makes caching possible,
  and it refuses the constructs it does not define (`fn`, `if`, a `-:`
  operator) as `construct-unknown` instead of silently ignoring them.
- Nine open questions are listed explicitly, including three KILN00 previously
  parked ("must settle at least"): the `BARE` word character set, `fn` bodies
  and call syntax, an `if` inside a phase, and a `-:` removal operator. Two
  decisions are proposed for review rather than assumed: rule files carry their
  own `kiln-rule 1` marker, and indentation width may be any consistent width.
- `code/specs/README.md` gains the KILN01 row and KILN00 §6 now points here.
