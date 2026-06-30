# Changelog — mathml

All notable changes to the Presentation-MathML frontend crate.

## [0.1.0] — 2026-06-30

### Added — PR-1: a Presentation-MathML reader → neutral `MathExpr` (PFE01 frontend #4)

- New crate `mathml`: `MathMl` implements `math_frontend::MathFrontend`, the **fourth pluggable
  parser frontend** after `latex`, `asciimath`, and `unicode-math`. It turns Presentation MathML
  (`<math>…</math>`) into the same neutral `MathExpr` the others produce, so a consumer supports
  MathML with **zero** change — the PFE01 promise, now demonstrated across an XML tree notation.
- **XML-subset event lexer** (no general XML engine needed): emits start/end/character-data events,
  ignoring attributes, namespace prefixes (`m:math` ≡ `math`), the XML declaration, comments, and
  DOCTYPE. Entities decode — the basic five, numeric `&#NN;`/`&#xHH;`, and the common operator
  entities (`&times;`, `&middot;`, `&le;`, `&ge;`, `&ne;`, `&plusmn;`, …); unknown entities are
  preserved verbatim rather than dropped.
- **Element coverage (PR-1):** `<mn>` (exact `Number`, tolerating `1,000`/`1.0`), `<mi>`
  (`Symbol`), `<mtext>` (`Text`), `<mo>` operators (`+ - * / = < > ≤ ≥ ≠ ≈ ≡ ± ∓` and entity
  spellings), `<mrow>` (a folded row with operator precedence relations < ±/∓/+− < ×÷, implicit
  multiplication of adjacent operands, unary signs, and `(`…`)` fences → `Group`), `<mfrac>` →
  `Frac`, `<msup>` → `Bin(Pow)`, `<msub>` → `Subscript`, `<msubsup>` → `Pow(Subscript(…))`,
  `<msqrt>`/`<mroot>` → `Root`. `<math>`/`<mstyle>`/`<mpadded>` are transparent wrappers.
- **Capabilities** (honest, checked by the shared `check_frontend` harness): fractions, roots,
  powers, relations, implicit_mul, text, plusminus. (Subscripts are core, no flag.)
- **Total, panic-free, spanned:** every input is `Ok(MathExpr)` or a `FrontendError` carrying a
  byte span. A `MAX_DEPTH` guard bounds **both** the element recursion and the `(`…`)` fence
  recursion, so adversarially deep input errors cleanly instead of overflowing the stack; the
  neutral tree drops iteratively (math-frontend ≥ 0.3.0). `#![forbid(unsafe_code)]`.
- `register_mathml`/`registry` helpers to install the frontend by name; 24 unit tests + a doctest
  cover leaves, precedence, implicit multiplication, fences, the built-up structures, namespace/
  attribute/declaration skipping, the cross-notation equivalences, spanned-error cases, the
  deep-nesting guard, the registry, and a conformance check.

### Deferred to PR-2
`<mtable>`/`<mtr>`/`<mtd>` → `Matrix`, `<mover>`/`<munder>` → over/under-sets, `<mfenced>`, and
named-function recognition (`<mi>sin</mi>` → `Call`).
