# TypeScript CAS Core Parity

## Goal

Port the foundational Python CAS packages to pure TypeScript so browser
MACSYMA can grow beyond the frontend/runtime shell without depending on Python
or Rust/WASM.

This phase adds the packages that other CAS modules naturally build on:

- `cas-pretty-printer`
- `cas-substitution`
- `cas-list-operations`
- `cas-simplify`

## Package Contracts

### `@coding-adventures/cas-pretty-printer`

Formats `@coding-adventures/symbolic-ir` trees as:

- Lisp prefix form for debugging IR shape.
- MACSYMA-style source text.
- Mathematica-style source text.
- Maple-style source text.

The printer owns dialect spelling and display sugar only; it does not evaluate
or simplify expressions.

### `@coding-adventures/cas-substitution`

Provides:

- `subst(value, variable, expr)` structural replacement.
- `substMany(rules, expr)` sequential structural replacement.
- `replaceAll(expr, rule)` single-pass rule replacement.
- Minimal `Blank` / `Pattern(name, inner)` matching needed for replacement
  rules while the full TypeScript `cas-pattern-matching` port is still pending.

### `@coding-adventures/cas-list-operations`

Provides pure list helpers over `List(...)` IR:

- `length`, `first`, `rest`, `last`
- `append` / `join`, `reverse`, `part`, `range`
- `mapList`, `applyList`, `select`, `sortList`, `flatten`

The functions operate on raw IR and leave VM integration to later runtime
wiring.

### `@coding-adventures/cas-simplify`

Provides the first TypeScript simplify layer:

- `canonical`
- `numericFold`
- `simplify`

The scope matches the Python package's foundational fixed-point shape:
canonicalization, integer numeric folding, and identity simplifications. More
advanced rational simplification operations remain separate package ports.

## Follow-Up Work

The next TypeScript CAS parity phases should port:

- `cas-pattern-matching` as a full package, then switch substitution to depend
  on it for richer matching.
- Algebra packages such as `cas-factor`, `cas-solve`, `cas-trig`,
  `cas-complex`, and `cas-number-theory`.
- Runtime handler wiring so MACSYMA calls can dispatch to these standalone CAS
  packages instead of only the baseline `symbolic-vm` handlers.
