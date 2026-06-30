# mathml

A **Presentation-MathML** reader that implements the [`math-frontend`](../math-frontend)
`MathFrontend` trait: it turns the XML notation `<math>…</math>` into the *same* neutral
`MathExpr` AST that the `latex`, `asciimath`, and `unicode-math` frontends produce.

It is the **fourth pluggable parser frontend** (see
[PFE01](../../../specs/PFE01-pluggable-parser-frontends.md)). The point of PFE01: a consumer
depends on the neutral `MathExpr`, never on a concrete notation, so supporting MathML required
**zero** change to any consumer — and now four genuinely different notations (a TeX macro
language, a terse linear notation, a Unicode-symbol notation, and an XML tree) all lower to one
tree.

## What it covers

| MathML | neutral `MathExpr` |
|--------|--------------------|
| `<mn>42</mn>` | `Number` (exact; `1,000` and `1.0` tolerated) |
| `<mi>x</mi>` | `Symbol` |
| `<mtext>kg</mtext>` | `Text` |
| `<mo>+ - * / = &lt; &gt; ≤ ≥ ≠ ≈ ≡ ± ∓</mo>` | the matching `Bin`/`Rel` operator |
| `<mrow>…</mrow>` | a folded row: operator precedence (relations < ± / +− < × ÷), implicit multiplication of adjacent operands, unary signs, and `(`…`)` fences → `Group` |
| `<mfrac>a b</mfrac>` | `Frac(a, b)` |
| `<msup>b e</msup>` | `Bin(Pow, b, e)` |
| `<msub>b s</msub>` | `Subscript(b, s)` |
| `<msubsup>b s e</msubsup>` | `Bin(Pow, Subscript(b, s), e)` |
| `<msqrt>x</msqrt>` / `<mroot>x n</mroot>` | `Root` (degree `None` / `Some(n)`) |
| `<mover>b a</mover>` / `<munder>b a</munder>` | `Overset { over: a, base: b }` / `Underset { under: a, base: b }` (PR-2) |
| `<munderover>b u o</munderover>` | `Underset { under: u, base: Overset { over: o, base: b } }` (PR-2) |
| `<mfenced>…</mfenced>` | `Group` over the folded contents (PR-2) |
| `<mtable><mtr><mtd>…</mtd></mtr></mtable>` | `Matrix(rows × cells)` (PR-2) |
| `<mi>sin</mi> … x` (applied) | `Call { func: Sin, arg: x }` (PR-3) |

`<math>`, `<mstyle>`, and `<mpadded>` are **transparent** wrappers (their children join the
surrounding row). Attributes, namespace prefixes (`m:math` ≡ `math`), the XML declaration,
comments, and DOCTYPE are ignored. Operator entity spellings (`&times;`, `&le;`, `&#xD7;`, …)
decode to the same operators as their literal glyphs. In `<mover>`/`<munder>` position an operator
glyph (`<mo>^</mo>`, `<mo>‾</mo>`, `<mo>→</mo>`) is read as an **annotation symbol**, not an infix
operator — so `<mover><mi>x</mi><mo>^</mo></mover>` is "x with a hat". An `<mi>` whose name is a known
function (sin, cos, ln, …) **applied to an argument** becomes a `Call`; the same name with no argument
stays a plain `Symbol`.

Deferred to PR-4: `<mfenced>` separator modelling (a comma-separated `(a, b)` list) — needs a neutral
*list* node the AST does not yet have; today a fence's contents fold as one row.

## Contract

`MathMl` implements `MathFrontend`: **total and panic-free** (every input is `Ok(MathExpr)` or a
spanned `FrontendError`), **pure**, and **honest** (its `capabilities()` match what it actually
emits — enforced by the shared `check_frontend` harness). Deeply-nested input is rejected with a
spanned error (a `MAX_DEPTH` guard on both the element recursion and the fence recursion) rather
than overflowing the stack, and the neutral tree it returns drops iteratively.

## Usage

```rust
use mathml::MathMl;
use math_frontend::{MathFrontend, MathExpr, BinOp};

let e = MathMl.parse("<math><mn>1</mn><mo>+</mo><mn>2</mn></math>").unwrap();
assert!(matches!(e, MathExpr::Bin(BinOp::Add, _, _)));

// `<mfrac>` means the same as LaTeX `\frac{1}{2}` and AsciiMath `1/2` — all `MathExpr::Frac`.
let f = MathMl.parse("<mfrac><mn>1</mn><mn>2</mn></mfrac>").unwrap();
assert!(matches!(f, MathExpr::Frac(_, _)));
```

Register it in a `FrontendRegistry` and parse by name:

```rust
let r = mathml::registry();
assert_eq!(r.names(), vec!["mathml"]);
let _ = r.parse("mathml", "<msup><mi>x</mi><mn>2</mn></msup>").unwrap();
```

## Where it fits

```
notation source ──▶ [frontend] ──▶ neutral MathExpr ──▶ one consumer (CAS, renderer, ADJ, …)
   LaTeX            latex
   AsciiMath        asciimath
   Unicode-math     unicode-math
   MathML  ◀── this crate
```

Standalone and reusable; `#![forbid(unsafe_code)]`; the only dependency is `math-frontend`.
