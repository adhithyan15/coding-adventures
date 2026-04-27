# cas-substitution

Replace symbols or sub-expressions inside a symbolic IR tree.

## Quick start

```python
from cas_substitution import subst
from symbolic_ir import IRApply, IRInteger, IRSymbol, ADD, POW

# subst(value, var, expr) — MACSYMA convention.
subst(IRInteger(2), IRSymbol("x"),
      IRApply(POW, (IRSymbol("x"), IRInteger(2))))
# IRApply(POW, (IRInteger(2), IRInteger(2)))
# (un-simplified — Simplify is a separate concern)
```

## API

- ``subst(value, var, expr)`` — MACSYMA convention. Returns a new IR
  with every occurrence of ``var`` replaced by ``value``.
- ``subst_many(rules, expr)`` — apply a list of ``(var, value)`` pairs
  in order.
- ``replace_all(expr, rule)`` — Mathematica convention. ``rule`` is
  any ``Rule(...)`` IR (from ``cas-pattern-matching``); applies it
  everywhere it matches.
- ``replace_all_many(expr, rules)`` — list-of-rules version.

The pattern-aware versions delegate to ``cas-pattern-matching``'s
matcher, so any pattern shape that engine supports can be used as the
LHS.

## Reuse story

Universal across CAS frontends. The same heads back Maxima's
``subst``, Mathematica's ``ReplaceAll`` (``/.``), Maple's ``subs``,
SymPy's ``subs``, Matlab's ``subs``. New frontends just add their
surface name to their runtime's name table; the head is always
``Subst`` or ``ReplaceAll``.

## Dependencies

- `coding-adventures-symbolic-ir`
- `coding-adventures-cas-pattern-matching`
