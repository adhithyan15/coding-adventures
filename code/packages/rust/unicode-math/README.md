# unicode-math — a Unicode plain-math frontend for `math-frontend`

The **third** pluggable parser frontend (after [`latex`](../latex) and [`asciimath`](../asciimath)).
It reads the math people and language models actually *type* with real glyphs —
`x² + y² = r²`, `√x`, `½`, `π·α`, `a ≤ b`, `2 ∓ 1` — and produces the **same** neutral
[`MathExpr`](../math-frontend) the other two frontends do. A consumer that already lowers
`MathExpr` gains this notation for **free**: adding it required **zero** change to any consumer.
That is the whole promise of [PFE01](../../../specs/PFE01-pluggable-parser-frontends.md), now
demonstrated across three genuinely different notations — a macro language (LaTeX), a terse
ASCII syntax (AsciiMath), and raw Unicode.

## What it parses (PR-1)

| Unicode math | → neutral `MathExpr` |
|--------------|----------------------|
| `42`, `3.14`, `6.022e23` | `Number` (exact — never `f64`) |
| `x`, `π`, `Σ`, `∞` | `Symbol` (`π`→`pi`, `Σ`→`Sigma`, `∞`→`infinity`) |
| `xy`, `2x`, `πα` | `Bin(Mul)` (juxtaposition ⇒ implicit `·`) |
| `x²`, `x⁻¹`, `x¹⁰`, `x^2` | `Bin(Pow)` (Unicode superscripts **or** the ASCII `^` operator) |
| `a₁`, `a₁²`, `a_i` | `Subscript` / `Pow(Subscript)` (Unicode glyph **or** ASCII `_`) |
| `∑_(i=1)^n i`, `∫_a^b f`, `∏ x` | `BigOp { op, lower, upper, body }` (`∑ ∏ ∫ ∮ ∐`) |
| `sin x`, `log(x)`, `arcsin x` | `Call { func, arg }` (named functions; longest-match, so `sinx` ⇒ `sin·x`) |
| `1/2`, `½`, `⅔` | `Frac` (built-up **and** vulgar-fraction glyphs) |
| `√x`, `∛x`, `∜x` | `Root { degree, radicand }` |
| `a + b`, `a − b`, `a × b`, `a ⋅ b`, `a ÷ b` | `Bin(Add/Sub/Mul/Div)` |
| `a ± b`, `a ∓ b` | `Bin(PlusMinus/MinusPlus)` |
| `a = b`, `a ≤ b`, `a ≠ b`, `a ≥ b`, `a ≈ b`, `a ≡ b` | `Rel` |
| `( … )`, `[ … ]`, `{ … }` | grouping (delimiter style dropped; meaning kept) |

`−` (U+2212 minus) and the ASCII `-` are both accepted, as are `×`/`⋅`/`*` for multiplication.
Greek and constant glyphs canonicalize to the **same** `Symbol` names the AsciiMath frontend
uses (`π` and `pi` both → `"pi"`), so the two notations agree on one neutral string.

## Example

```rust
use unicode_math::UnicodeMath;
use math_frontend::{MathFrontend, MathExpr, BinOp, RelOp};

let e = UnicodeMath.parse("x² + y² = r²").unwrap();
assert!(matches!(e, MathExpr::Rel(RelOp::Eq, _, _)));
// `x²` means the same as LaTeX `x^{2}` and AsciiMath `x^2` — all MathExpr::Bin(Pow, …).
assert_eq!(UnicodeMath.parse("½").unwrap(), UnicodeMath.parse("1/2").unwrap());
```

## Contract

It is **total and panic-free** — every input returns `Ok(MathExpr)` or a spanned
`FrontendError` (byte span into the original Unicode source), **pure** (no I/O), and **honest**:
`capabilities()` advertises exactly what it emits, enforced by the shared `check_frontend`
harness. Like every frontend it cannot be registered by `math-frontend` itself (that would be a
dependency cycle); a consumer registers it.

## Scope

PR-1 covered numbers/symbols/scripts/fractions/roots/relations/±∓; **PR-2** adds the big operators
`∑ ∏ ∫ ∮ ∐` (with optional bounds) and the explicit ASCII script operators `^`/`_`; **PR-3** adds
**named functions** (`sin cos tan … ln log exp`, longest-match so `sinx` ⇒ `sin·x`). **Out of scope**
(a clean spanned error, never a panic — tracked for PR-4): matrices and embedded `\text`. As in
AsciiMath there are no multi-letter variables: a non-function letter run like `xy` is the product
`x·y` (write distinct symbols, or use Greek/constant glyphs).
