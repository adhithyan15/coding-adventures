# Changelog — cas-substitution (Rust)

## [0.1.0] — 2026-04-27

### Added

- Initial Rust port of the Python `cas-substitution` package.
- `subst(value: IRNode, var: &IRNode, expr: IRNode) -> IRNode` — structural
  substitution: replaces every node structurally equal to `var` with a clone
  of `value`.  Works on any `IRNode` type as the search target (not just
  symbols).
- `subst_many(rules: &[(IRNode, IRNode)], expr: IRNode) -> IRNode` — sequential
  application of multiple `(var, value)` pairs.
- `replace_all(expr: IRNode, rule: &IRNode) -> IRNode` — pattern-aware
  top-down single-pass substitution using a `Rule(lhs, rhs)` from
  `cas-pattern-matching`.
- `replace_all_many(expr: IRNode, rules: &[IRNode]) -> IRNode` — sequential
  application of multiple rules.
- 20 integration tests + 4 doc-tests; all passing.
