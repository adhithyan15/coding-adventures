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

## What it covers (PR-1)

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

`<math>`, `<mstyle>`, and `<mpadded>` are **transparent** wrappers (their children join the
surrounding row). Attributes, namespace prefixes (`m:math` ≡ `math`), the XML declaration,
comments, and DOCTYPE are ignored. Operator entity spellings (`&times;`, `&le;`, `&#xD7;`, …)
decode to the same operators as their literal glyphs.

Deferred to PR-2: `<mtable>`/`<mtr>`/`<mtd>` → `Matrix`, `<mover>`/`<munder>` → over/under-sets,
`<mfenced>`, and named-function recognition (`<mi>sin</mi>` → `Call`).

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
