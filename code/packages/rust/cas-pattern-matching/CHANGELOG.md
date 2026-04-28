# Changelog — cas-pattern-matching (Rust)

## [0.1.0] — 2026-04-27

### Added

- Initial Rust port of the Python `cas-pattern-matching` package.
- `Bindings` — immutable `HashMap<String, IRNode>` with `bind` / `get` / `iter`.
- `match_pattern(pattern, target, bindings) -> Option<Bindings>` — structural
  matcher with `Blank()`, `Blank(T)`, `Pattern(name, inner)`, compound, and
  literal cases.
- Constructor helpers: `blank()`, `blank_typed(T)`, `named(name, inner)`,
  `rule(lhs, rhs)`, `rule_delayed(lhs, rhs)`.
- `apply_rule(rule, expr) -> Option<IRNode>` — single-shot rule application
  at the root.
- `rewrite(expr, rules, max_iter) -> Result<IRNode, RewriteCycleError>` — 
  bottom-up fixed-point rewrite with cycle detection.
- `RewriteCycleError` — returned when `max_iter` is exceeded.
- 38 integration tests + 9 doc-tests; all passing.
