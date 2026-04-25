# cas-pattern-matching — Pattern Variables and Rule Matcher

> **Status**: New spec. Defines a small Python package that adds
> Mathematica-style pattern matching on top of the symbolic IR.
> Parent: `symbolic-computation.md`. Foundation for `Rule`/`RuleDelayed`,
> `cas-substitution`, and `cas-simplify`.

## Why this package exists

The current `symbolic-vm` rule system uses closure-based predicate +
transform pairs. That works for hand-written rules ("if the head is
`Add` and one arg is `0`, drop it") but is verbose and inflexible.
Real CAS rule databases — `D[Sin[x_], x_] := Cos[x_]`,
`Sin[x_]^2 + Cos[x_]^2 -> 1` — use **pattern variables** that match any
expression and bind to a name. Once you have pattern variables you can
write rules that look like math.

This package is the matcher engine. It owns:

- The new IR nodes for patterns: `Blank`, `Pattern`, `Condition`,
  `BlankSequence`, `BlankNullSequence`, `OptionalPattern`.
- The matcher: given a pattern and a target, return a substitution
  binding pattern names to subterms (or `None` for no match).
- The rule type: `(pattern, replacement)` pairs, both `IRNode`.
- The rule applicator: walk an IR tree and apply rules until fixed
  point, with cycle detection.

## Reuse story

Every CAS that supports rule rewriting uses substantially the same
matcher. Mathematica's `Replace` / `ReplaceAll`, Maple's `subs`,
SymPy's `Wild` pattern, the Risch algorithm's intermediate rewriting
— they all reduce to "given a pattern with named holes, find a
substitution that makes pattern equal to target".

## Public interface

```python
from cas_pattern_matching import (
    # New IR nodes
    Blank,                  # _ (anonymous, matches anything)
    Pattern,                # x_ (named blank)
    BlankSequence,          # x__ (matches one or more)
    BlankNullSequence,      # x___ (matches zero or more)
    Condition,              # x_ /; predicate
    OptionalPattern,        # x_:default
    # Engine
    match,                  # match(pattern, target) -> Bindings | None
    Bindings,               # name → IRNode mapping
    apply_rule,             # apply one rule to one node
    rewrite,                # apply rules to fixed point
    Rule,                   # (lhs, rhs) pair
)

bindings = match(Pattern("x", Blank()), IRInteger(5))
# bindings == Bindings({"x": IRInteger(5)})

result = rewrite(expr, [Rule(lhs, rhs), ...])
```

### Pattern semantics

| Pattern              | Matches                              | Binds                |
|----------------------|--------------------------------------|----------------------|
| `Blank()`            | any single expression                | nothing              |
| `Blank(head)`        | any expression with that head        | nothing              |
| `Pattern("x", p)`    | whatever `p` matches                 | `x` → that subterm   |
| `BlankSequence()`    | 1+ args (only inside an `IRApply`)   | nothing              |
| `BlankNullSequence()`| 0+ args                              | nothing              |
| `Condition(p, pred)` | what `p` matches AND `pred` is true  | `p`'s bindings       |
| `OptionalPattern(p, default)` | what `p` matches, else default | `p`'s bindings       |

Same name appearing twice means same value: `Pattern("x", Blank()) +
Pattern("x", Blank())` matches `Add(a, a)` but not `Add(a, b)`.

## Algorithm

Recursive matcher with backtracking for sequence patterns:

```python
def match(pattern, target, bindings=Bindings()) -> Bindings | None:
    if isinstance(pattern, Blank):
        return bindings if matches_head(pattern, target) else None
    if isinstance(pattern, Pattern):
        sub = match(pattern.inner, target, bindings)
        if sub is None: return None
        if pattern.name in sub:
            return sub if sub[pattern.name] == target else None
        return sub.bind(pattern.name, target)
    if isinstance(pattern, Condition):
        sub = match(pattern.inner, target, bindings)
        return sub if sub and pattern.predicate(sub) else None
    if isinstance(pattern, IRApply):
        if not isinstance(target, IRApply): return None
        if pattern.head != target.head: return None
        return match_args(pattern.args, target.args, bindings)
    return bindings if pattern == target else None
```

`match_args` handles sequence patterns with backtracking: for every
position in the pattern's argument list, try every allowable target
slice and recurse. The first complete match wins.

## Heads added

| Head                | Arity | Meaning                             |
|---------------------|-------|-------------------------------------|
| `Blank`             | 0–1   | Anonymous wildcard.                 |
| `Pattern`           | 2     | Named binding for a sub-pattern.    |
| `BlankSequence`     | 0–1   | One-or-more sequence wildcard.      |
| `BlankNullSequence` | 0–1   | Zero-or-more sequence wildcard.     |
| `Condition`         | 2     | Pattern + predicate guard.          |
| `OptionalPattern`   | 2     | Pattern + default value.            |
| `Rule`              | 2     | `lhs -> rhs` immediate substitution.|
| `RuleDelayed`       | 2     | `lhs :> rhs` delayed substitution.  |
| `Replace`           | 2     | One-shot rule application.          |
| `ReplaceAll`        | 2     | Apply rule everywhere it matches.   |
| `ReplaceRepeated`   | 2     | Apply rules until fixed point.      |

## Test strategy

- Match a literal against a literal (`5` ⊨ `5`).
- Match a `Blank()` against anything.
- Match a named `Pattern` and read the binding.
- Repeated names: `f(x_, x_)` matches `f(2, 2)` not `f(2, 3)`.
- Sequence patterns: `f(x_, y__)` against `f(1, 2, 3, 4)` binds
  `x=1, y=Sequence(2, 3, 4)`.
- Condition predicates fire and reject correctly.
- Rule application: `Rule(Pow(x_, 0), 1)` rewrites `Pow(z, 0)` → `1`.
- Cycle detection: rewrite never infinite-loops on
  `Rule(x_, x_+0)` (this rule rewrites every node to itself + 0,
  growing forever — must detect and bail).
- Coverage: ≥95%.

## Package layout

```
code/packages/python/cas-pattern-matching/
  src/cas_pattern_matching/
    __init__.py
    nodes.py          # Blank, Pattern, Condition, ...
    matcher.py        # match()
    rewriter.py       # apply_rule, rewrite
    py.typed
  tests/
    test_match_literal.py
    test_match_pattern.py
    test_match_sequence.py
    test_rewrite.py
```

Dependencies: `coding-adventures-symbolic-ir`.

## Future extensions

- Attribute-aware matching: `Orderless` heads (`Add`, `Mul`) ignore
  argument order during matching.
- `Flat` heads: nested `Add`s are flattened during matching.
- `OneIdentity`: `Add(x)` matches `x`.
- These are exactly the Mathematica attribute system. Adding them
  later requires extending the matcher only — patterns and rules stay
  the same.
