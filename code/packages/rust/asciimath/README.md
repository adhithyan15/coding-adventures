# asciimath

An **[AsciiMath](http://asciimath.org/) parser** that implements the
[`math-frontend`](../math-frontend) `MathFrontend` trait — the **second** pluggable
frontend after [`latex`](../latex). It turns terse, human-typable AsciiMath
(`1/2`, `sqrt(x)`, `x^2 + y^2 = r^2`, `sum_(i=1)^n i`) into the **same neutral `MathExpr`**
the LaTeX frontend produces, so every consumer (a rule engine, a CAS, a renderer) lowers
*one* tree and gets both notations for free.

Spec: [`code/specs/ASM01-asciimath-frontend.md`](../../../specs/ASM01-asciimath-frontend.md)
(framework: [`PFE01`](../../../specs/PFE01-pluggable-parser-frontends.md)).

## Why it exists

```
   LaTeX     ─┐
 AsciiMath   ─┼─▶ MathExpr ─▶ (consumer lowers ONCE)
   MathML    ─┘   (neutral)
```

A math-capable model or a human can write `1/2` far more readily than `\frac{1}{2}`. For an
input→reasoning pipeline that translates a problem into a program, AsciiMath is a first-class
input notation — and adding it is exactly "register one more frontend," with **zero** change
to any consumer. That is the whole point of the framework; this crate is the proof with a
second notation.

## What it parses (PR-1 core)

| AsciiMath | → neutral `MathExpr` |
|-----------|----------------------|
| `42`, `3.14`, `6.022e23` | `Number` (exact — never `f64`) |
| `x`, `pi`, `alpha` | `Symbol` |
| `xy` | `x · y` (implicit product of single letters, the AsciiMath rule) |
| `a + b`, `a - b`, `a*b`, `a xx b`, `a -: b` | `Bin(Add/Sub/Mul/Div)` |
| `a b` (juxtaposition) | `Bin(Mul)` (implicit multiplication) |
| `1/2` | `Frac` |
| `x^2`, `a_i`, `a_i^2` | `Bin(Pow)` / `Subscript` / `Pow(Subscript)` |
| `sqrt(x)`, `sqrt x` | `Root { degree: None, .. }` |
| `root(3)(x)`, `root 3 x` | `Root { degree: Some(3), .. }` |
| `sin x`, `ln(x)` | `Call { func, arg }` |
| `a = b`, `x <= y`, `a != b` | `Rel` |
| `( .. )`, `[ .. ]`, `{ .. }` | `Group` |
| `"kg"` (text literal) | `Text` |

## PR-2 breadth (matrices + big operators)

| AsciiMath | → neutral `MathExpr` |
|-----------|----------------------|
| `[[a,b],[c,d]]`, `((1,0),(0,1))` | `Matrix(rows)` (rows of full-expression cells; `[…]` or `(…)` rows) |
| `[[a,b,c]]` | `Matrix` 1×3 (row vector) |
| `det[[a,b],[c,d]]` | `Call { func: Det, arg: Matrix(..) }` |
| `sum_(i=1)^n i`, `int_a^b f`, `prod x`, `lim_(x->0) f` | `BigOp { op, lower, upper, body }` |

`((a))` and `[[a]]` stay **grouping**, not a 1×1 matrix — a matrix must genuinely use commas
(≥2 rows, or a row with ≥2 cells). Big-operator bounds (`_`/`^`) attach to the operator in either
order; the body is the next atom (same convention as `sqrt`/functions). **Accents** (`hat x`, `vec x`)
are not yet supported: the neutral `MathExpr` has no `Accent` node, so adding one to `math-frontend`
is a prerequisite for an accents PR (see ASM01 §5).

It is **total and panic-free**: every input returns `Ok(MathExpr)` or a spanned
`FrontendError`. Recursion is depth-guarded (matrix nesting included); left-associative chains are
built with loops, so neither deep nesting nor long chains overflow the parser.

## Usage

```rust
use asciimath::AsciiMath;
use math_frontend::{MathFrontend, MathExpr, BinOp};

let e = AsciiMath.parse("1/2 + x^2").unwrap();
// 1/2  →  Frac;  x^2  →  Bin(Pow);  joined by Bin(Add)
assert!(matches!(e, MathExpr::Bin(BinOp::Add, _, _)));

// Same meaning as LaTeX's \frac{1}{2}: both lower to MathExpr::Frac.
assert_eq!(AsciiMath.parse("1/2").unwrap(), AsciiMath.parse("(1)/(2)").unwrap());
```

Registering it alongside LaTeX (a consumer concern — `math-frontend` can't depend on its own
frontends):

```rust
use math_frontend::FrontendRegistry;
let mut reg = FrontendRegistry::new();
reg.register(Box::new(asciimath::AsciiMath));
assert!(reg.parse("asciimath", "sqrt(x)").is_ok());
```

## Tests

```
cargo test -p asciimath
cargo clippy -p asciimath -- -D warnings
```
