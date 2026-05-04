# Changelog

## 0.2.0 — 2026-05-04

**Phase 22 — MACSYMA matchdeclare / defrule / apply1 / apply2 / tellsimp system.**

Two new modules added:

- `matchdeclare.py` — `MatchDeclareContext`: per-VM store that records
  which symbols are pattern variables and their type predicates.
  `compile_pattern(pattern)` walks an IR tree and substitutes declared
  variable symbols with `Pattern(name, Blank(constraint))` nodes ready
  for the existing `matcher.match` engine.  Supported predicates:
  `true`/`any` (unconstrained), `integerp`, `symbolp`, `floatp`,
  `rationalp`, `numberp`, `listp`, `stringp`.

- `defrule_engine.py` — `RuleStore`: per-VM named-rule dictionary that
  maps rule-name strings to compiled `Rule(lhs, rhs)` IR nodes.  Used
  by `defrule_handler` (store), `apply1_handler`, and `apply2_handler`
  (retrieve and apply).

Both classes exported from `cas_pattern_matching.__init__`.

---

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
