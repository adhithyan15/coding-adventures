# Changelog — symbolic-vm (Rust)

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
