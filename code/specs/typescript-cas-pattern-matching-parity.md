# TypeScript CAS pattern matching parity

## Goal

Port the Python/Rust `cas-pattern-matching` package to pure TypeScript so
browser-side CAS packages can share the same rewrite substrate.

## Scope

This slice mirrors the Rust crate surface:

- `Blank()` and `Blank(T)` wildcard nodes.
- `Pattern(name, inner)` named captures with consistency checks.
- `Rule(lhs, rhs)` and `RuleDelayed(lhs, rhs)` nodes.
- Immutable `Bindings` values.
- `matchPattern` structural matching over `symbolic-ir`.
- `applyRule` root rewriting with captured RHS substitution.
- `rewrite` bottom-up fixed-point rewriting with an iteration bound.

## Follow-up

Sequence wildcards and MACSYMA predicate-based declarations remain future
extensions, matching the current Rust crate's intentionally small surface.
