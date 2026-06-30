# Changelog — mathml

All notable changes to the Presentation-MathML frontend crate.

## [0.4.0] — 2026-06-30

### Added — PR-4: comma-separated `<mfenced>` lists lower to `Sequence`

- **`<mfenced>` with comma separators → `MathExpr::Sequence`.** A fence containing one or
  more top-level `<mo>,</mo>` separators (e.g. `(a, b, c)`) now emits `Sequence([a, b, c])`,
  preserving the list structure instead of folding the commas into a single row. Each
  segment between commas is itself folded to one expression (so an item may be compound, e.g.
  `(x+1, 2)`). The fence's `open`/`close`/`separators` attributes remain presentation and are
  dropped, as before.
- **A fence with NO commas is unchanged** — it still lowers to `Group` (an ordinary
  parenthesised sub-expression). An empty segment (leading/trailing/doubled comma) is
  malformed and reported as `empty MathML group`, so no list item is silently dropped.
- **Capabilities** add `sequences` (requires `math-frontend` 0.6.0), kept honest by
  `check_frontend` — the conformance corpus now includes a comma-separated `<mfenced>`.
- Tests: comma-separated sequence, compound items in a sequence, two-item pair, plus the
  retained no-comma → `Group` case.

### Deferred to a later PR
Semicolon separators and surfacing the fence's `open`/`close` delimiters as data.

## [0.3.0] — 2026-06-30

### Added — PR-3: named-function recognition (`<mi>sin</mi>` → `Call`)

- **Applied named functions → `MathExpr::Call`.** A function name arrives as a `<mi>` identifier
  (e.g. `<mi>sin</mi>`), usually with an invisible `<mo>&ApplyFunction;</mo>` before its argument
  that lowering already drops — so the function symbol sits directly adjacent to its argument. The
  row folder now recognises `sin x`, `cos(θ)`, `ln 2`, `log(x)`, … → `Call { func, arg }`, with the
  same recognised set (sin/cos/tan/cot/sec/csc, arc*, *h, ln/log/exp, min/max/gcd/lcm/det) and
  neutral `Func` values as the `unicode-math` and `asciimath` frontends — so all four notations agree
  on one tree.
- **Semantics:** the function takes ONE atom as its argument, then ordinary precedence resumes —
  `sin x y` → `(sin x)·y`. Nested names fold right — `sin cos x` → `Call(sin, Call(cos, x))`. A
  function name NOT followed by an operand is a plain `Symbol` (`sin` alone is the symbol `sin`,
  never an empty application); a one-letter variable is never a function.
- **Capabilities** add `functions`, kept honest by `check_frontend` (the conformance corpus now
  includes `sin x`).
- **Stack-safety regression** per the iterative-collection lesson: the leading run of function names
  is collected then folded (not recursed per name), so a *flat* 100 000-`<mi>sin</mi>` run parses
  (and the deep `Call` chain drops) without a stack overflow.
- 41 unit tests + doctest (was 35): applied function (with and without `&ApplyFunction;`), one-atom
  argument + implicit-mul, nested folding, bare-name-as-symbol, one-letter-not-a-function, and the
  long-run stack-safety test.

### Deferred to PR-4
`<mfenced>` separator modelling (a comma-separated `(a, b)` list) — needs a neutral *list* node the
AST does not yet have; today a fence's contents fold as one row.

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
