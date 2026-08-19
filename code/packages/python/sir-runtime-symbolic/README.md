# coding-adventures-sir-runtime-symbolic

SIR23 **Tier A** symbolic-expression + pattern/rewrite runtime for
**Semantic-IR-emitted Python**.

Implements exactly the seven Tier A `Expr` variants from the SIR23 spec
(`code/specs/SIR23-symbolic-pattern-semantic-ir.md`): `SymSymbol` /
`SymRational` / `SymApply` / `SymPatternBlank` / `SymPatternNamed` /
`SymRule` / `SymReplaceAll` — a term-tree type, a five-case structural
pattern matcher, and Wolfram's `/.` (`ReplaceAll`) / `//.`
(`ReplaceRepeated`) tree-rewrite operators. **Tier B is out of scope**:
there is no general expression evaluator here (no `Add`/`Sin`/`D`/etc.
numeric or symbolic folding) — a `SymApply` builds an inert term tree,
nothing more.

## Where it fits in the stack

```
Wolfram/Macsyma-family source ─▶ frontend ─▶ Semantic IR ─▶ semantic-ir-to-python ─▶ .py
                                                                                        │ imports
                                                                                        ▼
                                                        coding-adventures-sir-runtime-symbolic
                                                                    │ builds on
                                          ┌─────────────────────────┴─────────────────────────┐
                                          ▼                                                     ▼
                              coding-adventures-symbolic-ir                coding-adventures-cas-pattern-matching
                                (the IRNode term-tree type)                    (the structural matcher/rewriter)
```

This mirrors the **TypeScript** backend's *imported-package* model
(`semantic-ir-to-typescript` imports `@coding-adventures/
sir-runtime-symbolic`) rather than `semantic-ir-to-python`'s usual
inline-runtime convention for its OOP/exceptions/pairs concerns — the
Python backend emits an import of this package only when a module declares
the SIR23 `SymbolicExpr`/`PatternMatching`/`Rationals` features; pure
modules never gain the dependency. See the SIR23 spec's "Backend impact"
section, and this repo's own `coding-adventures-sir-runtime-array` (SIR22)
package for an identical precedent.

## Why this package is thin

The hard part — a term-tree type and a faithful structural pattern
matcher/substitution algorithm — already exists as two separate, already
*published* sibling packages this one builds on rather than re-invents:

- [`coding-adventures-symbolic-ir`](../symbolic-ir) — the term-tree type
  (`IRNode`: one of `IRSymbol`/`IRInteger`/`IRRational`/`IRFloat`/
  `IRString`/`IRApply`) this whole domain is built on.
- [`coding-adventures-cas-pattern-matching`](../cas-pattern-matching) — the
  `Bindings`/`match`/`apply_rule` five-case structural matcher (`Blank()`,
  `Blank(head)`, `Pattern(name, inner)`, compound-vs-compound, structural
  equality).

This package re-exports/wraps those primitives under SIR23's own
vocabulary, and adds exactly the three things neither sibling package has:
a Python-idiomatic constructor surface matching the published TypeScript
sibling package's own names, the `replace_all`/`replace_repeated` tree-wide
rewrite operators (neither dependency exposes this exact pair), and an
explicit recursion-depth cap on the parts of this package that walk a full,
potentially attacker-influenced runtime expression tree.

This is a direct structural port of the published TypeScript package
[`@coding-adventures/sir-runtime-symbolic`](../../typescript/sir-runtime-symbolic)
— same public surface, same algorithms — adapted to Python's own
`symbolic-ir`/`cas-pattern-matching` sibling packages (which differ in a
few naming/shape details from their TypeScript counterparts; see
`runtime.py`'s own module docstring for each spot that diverges and why).

## Security (DoS guards, CWE-674)

- **`MAX_TERM_DEPTH = 512`** caps `replace_all`'s (single-pass) and
  `replace_repeated`'s (fixed-point) tree WALK — not the matcher functions
  (`match_pattern`/`substitute`/`apply_rule`), whose recursion depth is
  bounded by a single rule's own author-written pattern/RHS shape, not by
  runtime data, so they need no cap. Fixed at the SAME value as every
  other Semantic-IR backend's own `MAX_TERM_DEPTH` (TypeScript,
  JavaScript, Ruby) — a compiled program hits the same depth ceiling no
  matter which backend emitted it.
- **`replace_repeated` also takes an independent `max_iterations` cap**
  (default 100) on its fixed-point loop — an unbounded `//.` is a
  guaranteed non-terminating program for some rule sets. This bounds CPU
  time, not stack depth, and the two guards are enforced independently.
- **The retry-on-fire step in `replace_repeated` is a local `while` loop,
  not a recursive call.** A naive port of `cas_pattern_matching.rewrite()`
  recurses once per rule firing at a single tree position, so its native
  stack depth is bounded only by `max_iterations`, not by any tree-depth
  cap — a caller passing a large `max_iterations` could exhaust the stack
  through that path alone. This package's `replace_repeated` instead loops
  locally when a rule fires, so `max_iterations` bounds only iteration
  *count*, never native recursion depth, however large a value is passed
  (verified by `test_survives_huge_max_iterations_without_a_stack_overflow`,
  a 50,000-firing cycling rule set with zero tree growth).
- No `eval`/`exec`/dynamic code execution anywhere in this package.
- `replace_all`/`replace_repeated` return an error *value*
  (`DepthLimitError`/`RewriteCycleError`) rather than raising directly;
  `unwrap()` converts either sentinel into a plain, catchable `ValueError`.

## API

| Export | Purpose |
|---|---|
| `IRNode`, `IRApply` | Re-exported term-tree types from `symbolic-ir`. |
| `sym(name)` / `int_(value)` / `number_node(value)` / `rational(numer, denom)` / `string_node(value)` / `apply(head, args)` | Leaf/compound term constructors. `int_` is re-exported as `int` from the package's top level (mirrors `sir-runtime-array`'s `range_`/`range` convention — the trailing-underscore name avoids shadowing the builtin inside `runtime.py` itself). |
| `BLANK` / `PATTERN` / `RULE` / `RULE_DELAYED` | The sentinel `IRSymbol` heads the pattern/rule vocabulary is built from. |
| `blank()` / `blank_typed(head)` | Wolfram `_` / `_h` pattern blanks. |
| `named(name, pattern)` | Wolfram `x_` / `x_h` named pattern variables. |
| `rule(lhs, rhs)` / `rule_delayed(lhs, rhs)` | Wolfram `->` / `:>` rewrite rules. |
| `is_blank` / `is_pattern` / `is_rule` | Structural predicates. |
| `match_pattern(pattern, target, bindings=None) -> Bindings \| None` | The five-case structural matcher. |
| `substitute(template, bindings) -> IRNode` | Replace named-pattern references in a template with their captured bindings. |
| `apply_rule(rule, expr) -> IRNode \| None` | Try one rule at the root of an expression. |
| `Bindings` | Immutable name→`IRNode` mapping a successful match produces. |
| `replace_all(expr, rules) -> IRNode \| DepthLimitError` | Wolfram `expr /. rules` — one pass, first match wins per subtree, no retry. |
| `replace_repeated(expr, rules, max_iterations=100) -> IRNode \| RewriteCycleError \| DepthLimitError` | Wolfram `expr //. rules` — fixed point. |
| `MAX_TERM_DEPTH` | The tree-walk recursion-depth cap (`512`). |
| `DepthLimitError` / `RewriteCycleError` | Error sentinels `replace_all`/`replace_repeated` return instead of raising. |
| `unwrap(result) -> IRNode` | Raise a `ValueError` on either sentinel; pass through a real `IRNode` unchanged. |

## Usage

```python
from coding_adventures_sir_runtime_symbolic import (
    apply, blank, int, named, replace_all, rule, sym, unwrap,
)

# x_ + 0 -> x_, applied once, everywhere in the tree
x_pat = named("x", blank())
drop_add_zero = rule(apply(sym("Add"), [x_pat, int(0)]), x_pat)
expr = apply(sym("Add"), [apply(sym("Add"), [sym("z"), int(0)]), int(0)])
unwrap(replace_all(expr, [drop_add_zero]))   # IRSymbol(name='z')
```

## Out of scope (Tier B)

No general expression evaluator: `Add`/`Sin`/`D`/user-function dispatch
folding is explicitly not implemented, per the SIR23 spec's own
"Explicitly out of scope" section. A `SymApply` builds an inert term tree;
nothing evaluates it.

## Development

```bash
uv venv && uv pip install -e ".[dev]"
.venv/bin/python -m ruff check src tests
.venv/bin/python -m mypy
.venv/bin/python -m pytest tests/ -v
```

## License

MIT
