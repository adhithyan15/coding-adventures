# ASM01 — AsciiMath pluggable frontend

**Status:** Specs-first. The **second** frontend built against
[PFE01](PFE01-pluggable-parser-frontends.md) (LaTeX, [LTX01](LTX01-full-latex-parser.md),
was the first). Proves the framework's central claim — *adding a notation is "register one
more frontend," with zero change to any consumer*.
**Author:** architecture pass, 2026-06-28.

## 1. Motivation

[AsciiMath](http://asciimath.org/) is a lightweight, human-typable math notation
(`1/2`, `sqrt(x)`, `sum_(i=1)^n i`, `x^2 + y^2 = r^2`). Math-capable models and humans
emit it readily — it is far terser than LaTeX. For the byte-provenance input→ADJ pipeline
([project north star]) the input LLM should be able to translate a problem written in
AsciiMath as easily as one written in LaTeX. So AsciiMath becomes frontend **#2**: a parser
that turns an AsciiMath string into the **same neutral `math_frontend::MathExpr`** the LaTeX
frontend produces. Two source strings that mean the same math — `\frac{1}{2}` (LaTeX) and
`1/2` (AsciiMath) — lower to the *same* tree, so every consumer treats them uniformly.

## 2. Crate

New crate `code/packages/rust/asciimath/` (workspace member, alphabetical-ish after
`latex`). Depends only on `math-frontend` (the neutral AST + trait). Zero other deps.

Public API:
- `pub struct AsciiMath;` implementing `math_frontend::MathFrontend`
  (`name() == "asciimath"`, `parse(src) -> Result<MathExpr, FrontendError>`, `capabilities()`).
- `pub fn parse(src: &str) -> Result<MathExpr, FrontendError>` — free-function form.
- `pub fn tokenize(src: &str) -> Result<Vec<Token>, FrontendError>` — the tokenizer, exposed.
- `pub use` of the re-exported neutral types for convenience.

The crate is **total and panic-free**: every input yields `Ok(MathExpr)` or a spanned
`FrontendError`. Recursion is depth-guarded (`MAX_DEPTH`) so deeply-nested input errors
rather than overflowing; left-associative chains (`a+a+…`, juxtaposition, `a/a/…`) are built
with loops, not recursion, so they do not consume call-stack per term.

## 3. Grammar — the PR-1 core subset

PR-1 implements a faithful, useful core; later PRs add breadth (§5). Precedence, low→high:

| Level | Construct | Neutral node |
|-------|-----------|--------------|
| relation | `= != < > <= >= ~~ -=` (and word forms `ne le ge`) | `Rel` (left-assoc) |
| add/sub | `+ -` | `Bin(Add/Sub)` |
| mul | `*`, `cdot`, `xx`, `-:`/`div`, **juxtaposition** (implicit) | `Bin(Mul/Div)` |
| frac | `a/b` | `Frac` (binds simple-with-scripts on each side; left-assoc) |
| unary | prefix `-`/`+` | `Unary(Neg/Pos)` |
| script | `a^b`, `a_b`, `a_b^c` | `Bin(Pow)` / `Subscript` / `Pow(Subscript)` |
| atom | number, symbol, function application, `sqrt`, `root(n)(x)`, group, `"text"` | … |

Atoms:
- **Number** — decimal numeral, kept exact (`Number`, never `f64`).
- **Identifier** (maximal `[A-Za-z]+` run), classified:
  - a **function** name (`sin cos tan cot sec csc sinh cosh tanh log ln exp det gcd lcm min max`)
    → `Call{func, arg}`, applied to the following simple expression;
  - a **known multi-letter constant** (`pi tau theta alpha beta gamma delta epsilon
    lambda mu phi omega sigma`, and `oo`/`infty` → `infinity`) → `Symbol(canonical)`;
  - the **unary keyword** `sqrt` → `Root{degree:None,…}`; the **binary keyword** `root`
    → `Root{degree:Some(n), radicand}` from `root(n)(x)` or `root n x`;
  - the **operator words** `xx cdot div`; otherwise
  - a **bare run**: a single letter → `Symbol(c)`; a multi-letter unknown run →
    the implicit product of its single-letter `Symbol`s (`xy` ⇒ `x·y`, the AsciiMath rule).
- **Group** — `( … )`, `[ … ]`, `{ … }` → `Group` (delimiter style dropped; meaning kept).
- **Text** — `"…"` → `Text`. (The `text(…)` keyword form is PR-2.)

### 3.1 Deliberate PR-1 limitations (documented, widened in §5)
Function names need a token boundary (`sin x`, `sin(x)`, not `sinx`). No matrices, big
operators, accents, or angle brackets yet. `/` binds the adjacent *simple* expressions
(AsciiMath's `S/S`), so `a b/c` is `a·(b/c)`; this is the AsciiMath grammar's intent and is
documented. None of these are *wrong* outputs — they are a smaller covered surface, declared
honestly via `Capabilities`.

## 4. Capabilities & conformance

`capabilities()` advertises exactly the PR-1 surface: `fractions, roots, powers, functions,
relations, implicit_mul, text` on; `matrices, big_operators` off (PR-2). The shared
`check_frontend` harness (PFE01 §5) verifies, over an AsciiMath sample corpus, that parsing
never panics, errors carry valid in-range spans, and declared capabilities match what is
actually emitted (no over-claiming). Notation-specific golden tests assert exact `MathExpr`
shapes for representative inputs.

## 5. Roadmap (later PRs)
- **PR-2 (shipped, v0.2.0):** matrices `[[a,b],[c,d]]` → `Matrix` (rows may use `[…]`/`(…)`;
  `((a))`/`[[a]]` stay grouping — a matrix must use commas; ragged rows error cleanly;
  `det[[…]]` binds the matrix); big operators `sum`/`prod`/`int`/`oint`/`coprod`/`lim` →
  `BigOp{op,lower,upper,body}` (optional `_`/`^` bounds either order, body = next atom). A new
  `Comma` token; `capabilities()` adds `matrices`+`big_operators`; matrix nesting is `MAX_DEPTH`-guarded.
- **PR-2b (shipped):** accents — `hat x`, `bar y`/`overline y`, `vec v`, `dot x`, `ddot x`,
  `tilde a`, `ul x`/`underline x` → `MathExpr::Accent{accent, body}` (a mark over the body, distinct
  from a function `Call`; body = next atom, like `sqrt`; synonyms normalise to one canonical name).
  Enabled by `math-frontend` 0.4.0's neutral `Accent` node; `capabilities()` adds `accents` (enforced
  by the conformance harness). Still deferred: angle/invisible brackets `(: … :)`, `{: … :}`.
- **PR-3a (shipped, v0.4.0):** the AsciiMath **symbol table** — completed lowercase Greek + variant
  glyphs, the visually-distinct uppercase Greek (`Gamma`…`Omega`), the blackboard number sets
  (`NN ZZ QQ RR CC` → `naturals`…`complexes`), arrow word-forms (`rarr`/`rightarrow`, `larr`, `harr`,
  `uarr`, `darr`, `implies`, `iff`, `mapsto`), set/logic operators (`notin`, `subset`, `subseteq`,
  `supset`, `supseteq`, `cup`→`union`, `cap`→`intersection`, `emptyset`, `forall`, `exists`, `aleph`),
  and misc. operators/decoration (`partial`, `nabla`/`grad`, `propto`, `perp`, `angle`, `deg`, the
  dots). Each lowers to `MathExpr::Symbol(canonical)`; symbol emission is not a `Capabilities` flag,
  so the table is purely additive (no consumer/conformance change).
- **PR-3b (shipped, v0.5.0):** finished the symbol-table surface — the bare English keyword spellings
  `in`/`and`/`or`/`not` (→ `Symbol`; verified safe inside big-operator bounds: `sum_(i in S)` parses
  as the juxtaposition `i · ∈ · S`), and AsciiMath's two-letter short forms `sub`→`subset`,
  `sube`→`subseteq`, `sup`→`supset`, `supe`→`supseteq`, `uu`→`union`, `nn`→`intersection`,
  `AA`→`forall`, `EE`→`exists`. Still additive (no `Capabilities`/conformance change). Behaviour note:
  `p("in")` changed from `i·n` to `Symbol("in")`.
- **PR-3c part 1 (shipped, v0.6.0):** punctuation arrows `->` and `=>` — recognized in the tokenizer
  and emitted as the existing `rightarrow`/`implies` identifiers, so they route through the PR-3a
  symbol table and lower to `Symbol` (agreeing with the word forms). Tokenizer-only, zero ripple; the
  single-char `-`/`=` are unaffected (`a - b` = `Bin(Sub)`, `a = b` = `Rel(Eq)`).
- **PR-3c part 2 (shipped, v0.7.0):** the **`text(…)` keyword form** alongside the `"…"` literal.
  The tokenizer captures the raw bytes up to the matching close paren (parens nest) and emits the same
  `TokenKind::Text`, so `text(kg)` and `"kg"` lower to an *identical* `MathExpr::Text` — tokenizer-only,
  no parser/`Capabilities`/conformance change (the `text` capability shipped in PR-1). The open paren
  must immediately follow `text`; otherwise `text` stays an ordinary identifier (a variable named
  `text`, `text (x)` with a space, or `textual` are all unchanged). An unterminated `text(` is a clean
  spanned error; byte-scanning for the matching paren is UTF-8-safe.
- **PR-3c part 3 (shipped, v0.8.0):** over/under-set **emission** — `overset(a)(b)` and the LaTeX
  synonym `stackrel(a)(b)` → `MathExpr::Overset{over,base}`; `underset(a)(b)` → `MathExpr::Underset
  {under,base}`. Each keyword takes two atoms (annotation then base, the `root(n)(x)` convention; the
  paren-free `stackrel a b` form works too). A centered mark over/under the base, distinct from
  `Pow`/`Subscript`; enabled by `math-frontend` 0.5.0's neutral `Overset`/`Underset` node (added in the
  prerequisite PR, the same staging as the `Accent` node → accent emission). `capabilities()` adds
  `oversets`, enforced by the conformance harness.
- **PR-3c remainder** (its own follow-up, structural): **longest-match tokenization** (so `sinx`
  splits as `sin`·`x`, a deeper lexer change).

## 6. Non-goals
Evaluation, simplification, rendering — none are a frontend's job (PFE01 §7). Lowering
`MathExpr` into any engine IR is a consumer concern. Like every frontend, `asciimath` cannot
be registered by `math-frontend` itself (that would be a dependency cycle); a consumer (or a
future aggregating crate) registers it alongside `latex`.
