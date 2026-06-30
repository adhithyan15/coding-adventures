# PFE01 — Pluggable Parser Frontends

**Status:** Specs-first. Defines the framework; LaTeX (see [LTX01](LTX01-full-latex-parser.md))
is the first frontend built against it.
**Author:** architecture pass, 2026-06-26.

## 1. Motivation

A reasoning/adjudication system must accept mathematics and structured knowledge in the
**many notations people and models actually write**: LaTeX, AsciiMath, MathML,
Unicode/plain math, MathJSON, content-MathML, spreadsheet formulae, … A model trained on
math emits LaTeX by reflex; a chemist writes one notation, an economist another. We must
**not** hard-code each notation into every consumer (the ADJ language, a CAS, a renderer).

Instead: a **pluggable frontend registry**. A *frontend* is a parser that turns a source
string in one notation into a **common neutral math AST**. Consumers depend on the neutral
AST, never on a specific notation. Adding support for a new notation is "register one more
frontend" — zero changes to consumers. LaTeX is frontend #1; the framework anticipates the
rest.

This mirrors how the workspace already plugs in interchangeable implementations behind a
common contract (grammar-driven language frontends; the constraint-solver backends behind
one solver interface; the many `*-parser` crates). Frontends are the same idea for input
notations.

## 2. The neutral target: `MathExpr`

All frontends produce the **same** AST, so a consumer lowers it **once**. The neutral tree
is notation-agnostic (it is *not* LaTeX-shaped):

```rust
pub enum MathExpr {
    Number(Number),                 // exact-preserving numeric literal (see §2.1)
    Symbol(String),                 // a variable or named constant: "x", "pi", "alpha"
    Bin(BinOp, Box<MathExpr>, Box<MathExpr>),   // Add Sub Mul Div Pow PlusMinus MinusPlus
                                                //   (Mul carries no surface style — \cdot vs
                                                //   \times vs juxtaposition all become Mul;
                                                //   PlusMinus/MinusPlus = ± / ∓)
    Unary(UnaryOp, Box<MathExpr>),              // Neg, Pos
    Frac(Box<MathExpr>, Box<MathExpr>),         // numerator / denominator (a Div with intent)
    Binom(Box<MathExpr>, Box<MathExpr>),        // binomial coefficient C(n,k) (no division bar)
    Root { degree: Option<Box<MathExpr>>, radicand: Box<MathExpr> },  // nth root
    Call { func: Func, arg: Box<MathExpr> },    // sin, cos, ln, exp, … (named functions)
    BigOp { op: BigOp, lower: Option<Box<MathExpr>>, upper: Option<Box<MathExpr>>,
            body: Box<MathExpr> },              // sum, prod, int, lim with bounds
    Subscript(Box<MathExpr>, Box<MathExpr>),    // indexing: a_i  (distinct from Pow)
    Rel(RelOp, Box<MathExpr>, Box<MathExpr>),   // = < > <= >= != ≈ ≡
    Group(Box<MathExpr>),                       // explicit grouping (parens/braces)
    Text(String),                               // \text{…}: prose inside math (units, labels)
    Matrix(Vec<Vec<MathExpr>>),                 // rows × cols
    Accent { accent: String, body: Box<MathExpr> }, // diacritic over body: \hat{x}, \bar{y},
                                                //   \vec{v}, … (distinct from a Call: a mark,
                                                //   not a named-function application)
    Overset  { over:  Box<MathExpr>, base: Box<MathExpr> }, // \overset{a}{b}, \stackrel{a}{R}:
    Underset { under: Box<MathExpr>, base: Box<MathExpr> }, //   a full expr stacked over/under a
                                                //   base — generalises Accent; distinct from
                                                //   Pow/Subscript (centered, not raised/lowered)
}
```

### 2.1 Numbers are exact-preserving

`Number` keeps the literal form (an arbitrary-precision rational or the original digit
string), **not** an `f64`. Lossy float conversion is a *consumer's* choice at lowering
time, never the frontend's — a frontend must not silently round `0.1`. (This protects the
"engine does the math, exactly" principle downstream.)

### 2.2 What the neutral AST deliberately omits

Presentation-only distinctions that don't change meaning are **normalized away**:
`\times` / `\cdot` / juxtaposition → `Mul`; `\dfrac` / `\tfrac` / `\frac` → `Frac`; font
styling (`\mathbf`, `\mathrm`) → the bare symbol (with the styled spelling preserved only
inside `Text`). Two source strings that mean the same math produce the same `MathExpr`.
This is what lets a consumer compare/compute without caring which notation was typed.

## 3. The frontend contract

```rust
pub trait MathFrontend {
    /// Stable identifier, e.g. "latex", "asciimath", "mathml".
    fn name(&self) -> &str;

    /// Parse one source string in this notation into the neutral AST.
    fn parse(&self, src: &str) -> Result<MathExpr, FrontendError>;

    /// Which neutral constructs this frontend can currently emit (so a consumer can
    /// gate before relying on, say, matrices). See §5.
    fn capabilities(&self) -> Capabilities;
}

pub struct FrontendError {
    pub frontend: String,        // which frontend raised it
    pub message: String,
    pub span: (usize, usize),    // half-open byte span into `src`
}
```

A frontend MUST be **total and panic-free**: every input yields either a `MathExpr` or a
spanned `FrontendError`. A frontend MUST be **pure**: no I/O, no global state, no network.

### 3.1 Registry

```rust
pub struct FrontendRegistry { /* name -> Box<dyn MathFrontend> */ }
impl FrontendRegistry {
    pub fn with_builtins() -> Self;             // registers latex (+ future frontends)
    pub fn register(&mut self, f: Box<dyn MathFrontend>);
    pub fn get(&self, name: &str) -> Option<&dyn MathFrontend>;
    pub fn parse(&self, name: &str, src: &str) -> Result<MathExpr, FrontendError>;
}
```

Lookup is by `name`. Unknown name → a `FrontendError` naming the unknown frontend and
listing the registered ones (never a panic).

## 4. Consumer surface (informative — not built here)

How a consumer *selects* a frontend is the consumer's concern. The expected ADJ surface is
a **tagged literal** keyed by frontend name — `latex"\frac{a}{b}"`, `asciimath"a/b"` — so
the same `let` can ingest any registered notation. That wiring (and any lowering of
`MathExpr` into the engine's compute IR, plus an engine power op for `^`/roots) is a
**separate** effort that *consumes* this framework. Per the standing direction, the parser
work and the consumption work are kept distinct.

## 5. Capabilities & conformance

`Capabilities` is a bitset over `MathExpr` variants (plus sub-features like
`implicit_mul`, `big_op_bounds`, `matrices`, `plusminus`, `binomials`, `accents`). A consumer
can ask "does this frontend emit matrices yet?" and gate gracefully instead of failing at
parse time. The conformance harness flags any frontend that *emits* a variant (e.g. an
`Accent`) it did not *declare*, so a capability is an enforced promise, not a hope.

**Conformance harness (shared):** a notation-agnostic test battery asserts, for every
registered frontend, that (a) parsing its sample corpus never panics; (b) errors carry a
valid in-range span; (c) declared capabilities match what the frontend actually emits. Each
frontend additionally ships notation-specific golden tests.

## 6. Crates

- `math-frontend` — this framework: `MathExpr`/`Number`/op enums, the `MathFrontend` trait,
  `FrontendRegistry`, `FrontendError`, `Capabilities`, the shared conformance harness.
  Zero deps.
- `latex` — the first frontend (full LaTeX; see [LTX01](LTX01-full-latex-parser.md)). It is
  a standalone full-LaTeX parser in its own right and *also* implements `MathFrontend` via a
  thin adapter (math nodes of its document AST → `MathExpr`).
- Future: `asciimath`, `mathml`, `unicode-math`, … each a small crate implementing the
  trait. None require changes to consumers.

## 7. Non-goals

Evaluation, simplification, and rendering are **not** frontend concerns — a frontend only
*parses*. Lowering `MathExpr` into any particular engine IR belongs to the consumer.

## 8. Delivery

1. `math-frontend` crate: neutral AST + trait + registry + error + capabilities + harness.
2. `latex` crate implements `MathFrontend` (after LTX01's parser exists).
3. Each later notation = one more small frontend crate.

(LaTeX itself is staged per [LTX01](LTX01-full-latex-parser.md); this framework can land in
parallel since it only depends on the neutral AST.)
