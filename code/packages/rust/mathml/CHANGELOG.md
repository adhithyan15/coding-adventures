# Changelog — mathml

All notable changes to the Presentation-MathML frontend crate.

## [0.2.0] — 2026-06-30

### Added — PR-2: tables, over/under-sets, and fences (structural breadth)

- **`<mtable>`/`<mtr>`/`<mtd>` → `MathExpr::Matrix`.** A dedicated structural parser walks the rows
  and cells directly (a table's children are rows, a row's are cells — neither is an operand), so a
  2×2 `<mtable>` becomes `Matrix([[1,2],[3,4]])`. Each `<mtd>` cell folds a full expression (not
  just an atom). Stray non-`<mtr>` content in a table, or non-`<mtd>` content in a row, is a spanned
  error; an empty cell becomes an empty `Symbol`.
- **`<mover>`/`<munder>`/`<munderover>` → `Overset`/`Underset`.** MathML's base-first order maps to
  `Overset { over, base }` / `Underset { under, base }`; `<munderover>` nests under-most outside
  (`Underset { under, base: Overset { over, base } }`). Because an over/under annotation is commonly
  an operator *glyph* (a hat `^`, bar `‾`, arrow `→`, brace `⏞`), these read their arguments with a
  script-args reader that accepts a lone `<mo>` as an annotation **symbol** rather than rejecting it
  as a bare infix operator.
- **`<mfenced>` → `Group`.** A parenthesised group folds its contents to one row wrapped in `Group`
  (its `open`/`close`/`separators` attributes are presentation, dropped like all attributes; a
  comma-separated list folds as one row — separators are not yet modelled).
- **Capabilities** grow to add `matrices` and `oversets`, kept honest by `check_frontend` (the
  conformance sample corpus now includes a table, an over/under-set, and a fenced group).
- **Stack-safety regression** per the unary-overflow lesson: a *flat* 50 000-row `<mtable>` parses
  (and the `Matrix` drops) without a stack overflow, since the structural table/row parsing is
  iterative, not recursive-per-row.
- 35 unit tests + a doctest (was 24): adds tables, table-cell folding, table well-formedness errors,
  over/under/under-over sets, mover arity, fenced groups, and the wide-table stack-safety test.

### Deferred to PR-3
Named-function recognition (`<mi>sin</mi>` → `Call`), and `<mfenced>` separator modelling.

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
