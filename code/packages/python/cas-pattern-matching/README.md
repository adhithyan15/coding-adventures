# cas-pattern-matching

Mathematica-style pattern variables and a rule rewriter for the
symbolic IR.

## Quick start

```python
from cas_pattern_matching import (
    Blank, Pattern, Rule, match, apply_rule, rewrite,
)
from symbolic_ir import IRApply, IRInteger, IRSymbol, ADD, POW

# A pattern: x_  (named x, matches any single expression)
x_pat = Pattern("x", Blank())

# Match it against the literal 5.
bindings = match(x_pat, IRInteger(5))
# {"x": IRInteger(5)}

# Repeated names: same value required.
sym_pat = IRApply(ADD, (Pattern("x", Blank()), Pattern("x", Blank())))
match(sym_pat, IRApply(ADD, (IRSymbol("a"), IRSymbol("a"))))   # ok
match(sym_pat, IRApply(ADD, (IRSymbol("a"), IRSymbol("b"))))   # None

# Rewrite: x^0 → 1
rule = Rule(IRApply(POW, (Pattern("x", Blank()), IRInteger(0))), IRInteger(1))
rewrite(IRApply(POW, (IRSymbol("z"), IRInteger(0))), [rule])
# IRInteger(1)
```

## Phase B scope

This package ships the foundational matcher:

- **`Blank()`** — anonymous wildcard, matches any single expression.
- **`Blank(head)`** — wildcard matching any expression with a given
  head (e.g., `Blank("Integer")` matches any integer literal — note
  the head check inspects `IRApply.head` for compounds; literals are
  matched by their type name `Integer` / `Symbol` / `Rational` / etc).
- **`Pattern(name, inner)`** — names whatever `inner` matches. Repeated
  names enforce equality.
- **`Rule(lhs, rhs)`** — immediate-substitution rule.
- **`apply_rule(rule, expr)`** — one-shot rule application at the root.
- **`rewrite(expr, rules, max_iterations=100)`** — apply rules
  recursively (post-order walk) until fixed point.

Deferred to subphases:

- `BlankSequence` / `BlankNullSequence` (`x__` / `x___`) — sequence
  patterns inside `IRApply` argument lists. Need a backtracking matcher.
- `Condition` — predicate guards. Need a registered-predicate
  table (since IR is hashable, predicates can't live inside the IR).
- `OptionalPattern` — patterns with default values.
- Mathematica attribute system (Orderless / Flat / OneIdentity).

## Reuse story

Every CAS that supports rule rewriting uses substantially the same
matcher. Mathematica's `Replace` / `ReplaceAll`, Maple's `subs`, SymPy's
`Wild` pattern — they all reduce to this operation. Future packages
(`cas-substitution`, `cas-simplify`, etc.) consume this matcher
directly.

## Dependencies

- `coding-adventures-symbolic-ir`
