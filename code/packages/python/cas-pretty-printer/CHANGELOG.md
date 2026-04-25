# Changelog

## 0.1.0 — 2026-04-25

Initial release.

- `Dialect` protocol and `BaseDialect` ABC.
- Walker handles every IR node type (`IRSymbol`, `IRInteger`,
  `IRRational`, `IRFloat`, `IRString`, `IRApply`).
- Operator precedence and associativity tracking; parens inserted
  only when required.
- Surface-syntax sugar: `Add(x, Neg(y)) → x - y`, `Mul(x, Inv(y)) →
  x / y`, `Mul(-1, x) → -x`.
- `MacsymaDialect`, `MathematicaDialect`, `MapleDialect`,
  `LispDialect` ship out of the box.
- `register_head_formatter` hook for downstream packages to teach
  the printer about new heads (Matrix, Determinant, Limit, etc.).
- Type-checked (`py.typed`); ruff- and mypy-clean.
