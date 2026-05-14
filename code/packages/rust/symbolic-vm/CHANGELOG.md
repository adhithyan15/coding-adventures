# Changelog — symbolic-vm (Rust)

## Unreleased

### Added

- `SymbolicBackend` now installs canonical `Factor` handling backed by
  `cas-factor`, including common-symbolic-factor extraction for additive
  multivariate expressions before univariate integer factorization.
- `Factor` extracts the greatest common integer content (GCD of all term
  coefficients) and intersection of common symbolic powers before attempting
  specific pattern matches. For example `factor(2*x + 4*y)` → `2*(x + 2*y)`,
  `factor(2*x*y + 2*x*z)` → `2*x*(y + z)`, and `factor(2*x^2*y - 2*y)` →
  `2*y*(x+1)*(x-1)` (the univariate residual is factored recursively).
- `Factor` recognises bivariate perfect-square trinomials such as
  `x^2 + 2*x*y + y^2` and rewrites them as `(x + y)^2`.
- `Factor` recognises bivariate difference-of-squares expressions such as
  `x^2 - y^2` and rewrites them as `(x - y) * (x + y)`.
- `Factor` recognises bivariate cubic identities such as `x^3 - y^3` and
  `x^3 + y^3`, rewriting them to their canonical linear/quadratic products.
- `Factor` recognises four-term bilinear grouping such as
  `x*y + x*z + y + z` and rewrites it as `(x + 1) * (y + z)`.
- `Factor` recognises four-term bivariate perfect-cube expansions such as
  `x^3 + 3*x^2*y + 3*x*y^2 + y^3` and `x^3 - 3*x^2*y + 3*x*y^2 - y^3`,
  rewriting them as `(x + y)^3` and `(x - y)^3` respectively.
- `SymbolicBackend` installs a `D` derivative handler for symbolic-only
  differentiation of arithmetic, elementary, hyperbolic, and inverse
  hyperbolic expressions; `StrictBackend` continues to reject `D` as an
  unknown head.
- Numeric and symbolic handlers for reciprocal hyperbolic heads `Coth`,
  `Sech`, and `Csch`, including `sech(0) = 1` and undefined-at-zero checks for
  `coth`/`csch`.

## [0.1.0] — 2026-04-27

### Added

- Initial Rust port of the Python `symbolic-vm` package.
- `Backend` trait with `lookup`, `bind`, `on_unresolved`, `on_unknown_head`,
  `handler_for`, `rules`, `hold_heads`.
- `Handler` type alias: `Arc<dyn Fn(&mut VM, IRApply) -> IRNode + Send + Sync>`.
- `VM` struct with `eval(IRNode) -> IRNode` and `eval_program(Vec<IRNode>) -> Option<IRNode>`.
- `BaseBackend` — shared environment + held-heads for the two reference backends.
- `StrictBackend` — numeric-only evaluator; panics on unbound symbols or unknown heads.
- `SymbolicBackend` — Mathematica-style; unbound names stay as free variables;
  algebraic identities (`x+0→x`, `x*1→x`, `0*x→0`, `x^0→1`, etc.) are applied.
- Full handler table (34 handlers): `Add`, `Sub`, `Mul`, `Div`, `Pow`, `Neg`, `Inv`,
  `Sin`, `Cos`, `Tan`, `Exp`, `Log`, `Sqrt`, `Atan`, `Asin`, `Acos`, `Sinh`, `Cosh`,
  `Tanh`, `Asinh`, `Acosh`, `Atanh`, `Equal`, `NotEqual`, `Less`, `Greater`,
  `LessEqual`, `GreaterEqual`, `And`, `Or`, `Not`, `If`, `Assign`, `Define`, `List`.
- Exact rational arithmetic: `Numeric` enum preserving `Int(i64)`, `Rat(i64, i64)`,
  `Float(f64)` intermediate values; checked overflow falls back to `Float`.
- User-defined function support via `Define(name, List(params), body)` records,
  evaluated by substitution.
- 52 integration tests + 2 doc-tests; all passing.
