# Changelog

## 0.1.0 — 2026-04-18

Initial release.

- Generic tree-walking `VM` over `symbolic_ir` nodes.
- `Backend` ABC with `lookup`, `bind`, `on_unresolved`,
  `on_unknown_head`, `rules`, `handlers`, `hold_heads`.
- `StrictBackend`: Python-like semantics; raises on unbound names or
  unknown heads; requires arithmetic operands to be numeric.
- `SymbolicBackend`: Mathematica-like semantics; leaves unbound names
  as free symbols; applies identity/zero laws; knows calculus.
- Shared handler table for arithmetic (`Add`, `Sub`, `Mul`, `Div`,
  `Pow`, `Neg`, `Inv`), elementary functions (`Sin`, `Cos`, `Exp`,
  `Log`, `Sqrt`), comparisons, logic, assignment, and definition.
- `D` handler on the symbolic backend implements sum, difference,
  product, quotient, power, and chain rules.
- User-defined functions via `Define(name, List(params), body)` —
  the VM detects the bound record and performs parameter substitution.
- `If` is a held head; only the chosen branch is evaluated.
- End-to-end tests cover the full pipeline (MACSYMA source → tokens
  → AST → IR → evaluated result).
