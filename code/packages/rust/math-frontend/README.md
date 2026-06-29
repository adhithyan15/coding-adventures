# math-frontend

A **pluggable parser-frontend framework** for mathematics, built as a standalone,
dependency-free Rust crate. It is the shared substrate that lets a reasoning system accept
math in *many* notations — LaTeX, AsciiMath, MathML, Unicode/plain math — without
hard-coding any of them into its consumers.

Spec: [`code/specs/PFE01-pluggable-parser-frontends.md`](../../../specs/PFE01-pluggable-parser-frontends.md).

## The idea

A **frontend** is a parser for one notation that produces **one common neutral AST**,
`MathExpr`. Every consumer (a rule/inference engine, a computer-algebra system, a renderer)
depends on `MathExpr`, never on a specific notation. So:

```
   LaTeX  ─┐
 AsciiMath ─┼─▶ MathExpr ─▶ (consumer lowers ONCE)
   MathML ─┘
```

Adding a notation later is "register one more frontend" — zero consumer changes.

## What's here

| Piece | Role |
|-------|------|
| `MathExpr` (+ `Number`, `BinOp`, `Func`, `BigOp`, `RelOp`, …) | the notation-agnostic AST — includes `BinOp::PlusMinus`/`MinusPlus` (± / ∓) and `MathExpr::Binom` (binomial coefficient) |
| `MathFrontend` | the contract a notation parser implements |
| `FrontendError` / `Capabilities` | spanned errors; what a frontend can emit |
| `FrontendRegistry` | look up a frontend by name and parse through it |
| `check_frontend` | shared conformance harness (total/panic-free, well-formed errors, honest capabilities) |

### Two design rules worth knowing

- **Numbers are exact-preserving.** `Number` keeps a literal's exact decimal value
  (normalized `±digits×10^exp`), so `1`, `1.0`, `01`, `1e0` compare equal and `0.1` is
  never silently rounded. Lossy `to_f64()` is an explicit choice the *consumer* makes.
- **Presentation normalizes away; meaning is kept.** `\times`, `\cdot`, and juxtaposition
  all become `Mul`; `\frac`/`\dfrac`/`/` all mean division. Two strings that mean the same
  math produce the same `MathExpr`.

## Status

This crate is the framework. There are **no built-in frontends yet** —
`FrontendRegistry::with_builtins()` is empty by design until LaTeX (the first frontend,
its own `latex` crate) implements `MathFrontend` and registers here. See
[`LTX01`](../../../specs/LTX01-full-latex-parser.md).

## Usage

```rust
use math_frontend::{FrontendRegistry, MathFrontend, MathExpr, Number, Capabilities, FrontendError};

struct Int;
impl MathFrontend for Int {
    fn name(&self) -> &str { "int" }
    fn parse(&self, s: &str) -> Result<MathExpr, FrontendError> {
        Number::parse(s).map(MathExpr::Number)
            .ok_or_else(|| FrontendError::new("int", "not an integer", (0, s.len())))
    }
    fn capabilities(&self) -> Capabilities { Capabilities::none() }
}

let mut reg = FrontendRegistry::new();
reg.register(Box::new(Int));
assert_eq!(reg.parse("int", "42").unwrap(), MathExpr::Number(Number::from_i64(42)));
```

## Tests

```
cargo test -p math-frontend
cargo clippy -p math-frontend -- -D warnings
```
