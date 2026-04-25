# Changelog

## 0.1.0 — 2026-04-25

Initial release — Phase B foundation.

- IR sentinel heads: `Blank`, `Pattern`, `Rule`, `RuleDelayed`,
  `Replace`, `ReplaceAll`, `ReplaceRepeated`.
- Helper constructors: ``Blank()``, ``Blank(head)``,
  ``Pattern(name, inner)``, ``Rule(lhs, rhs)``.
- Matcher: structural recursion with named-binding equality check.
- ``apply_rule(rule, expr)`` — one-shot at the root.
- ``rewrite(expr, rules, max_iterations=100)`` — bottom-up walk to
  fixed point with cycle detection.
- Type-checked, ruff- and mypy-clean.

Deferred to follow-ups: ``BlankSequence``/``BlankNullSequence``,
``Condition``, ``OptionalPattern``, attribute-aware matching
(Orderless / Flat / OneIdentity).
