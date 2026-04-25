# PR16: Prolog Expansion Pipeline

## Summary

This batch adds the first explicit expansion pipeline above the parsers.

`prolog-core` now has shared metadata models for `term_expansion/2` and
`goal_expansion/2`, and `prolog-loader` applies those declarations during load
as a deterministic rewrite phase over parsed clauses, queries, and
initialization goals.

## Goals

- preserve parser determinism and keep parsing side-effect free
- collect structured `term_expansion/2` and `goal_expansion/2` declarations in
  shared Prolog-facing runtime objects
- apply term expansion to loaded clause terms
- apply goal expansion to loaded queries and initialization goals
- support repeated expansion passes until the loaded source reaches a fixed
  point
- support list-valued `term_expansion/2` results so one source term can expand
  into multiple clauses
- fail clearly when an expansion produces invalid executable output

## Design

`prolog-core` adds:

- `PrologTermExpansion`
- `PrologGoalExpansion`
- `term_expansion_from_directive(...)`
- `goal_expansion_from_directive(...)`

The existing `PredicateRegistry` now also retains ordered term and goal
expansion declarations, so the operator-aware parser can continue feeding every
directive through one shared directive-collection path.

`prolog-loader` adds:

- `PrologExpansionError`
- `apply_expansion_directives(...)`

Loaders now:

1. parse with a dialect frontend
2. collect directive metadata
3. build a `LoadedPrologSource`
4. run explicit expansion passes over the loaded executable artifacts

The first expansion pipeline is intentionally collection-oriented:

- facts expand as bare callable source terms
- rules expand as `:-(Head, Body)` source terms
- queries expand as Prolog goal terms
- initialization terms expand from their original directive argument terms

## Non-goals

- exact source-order compile-time hook semantics
- term expansion of directives themselves
- recursive sub-goal rewriting inside arbitrary clause bodies
- `expand_term/2` style runtime APIs
- module-scoped expansion visibility rules
- expansion hooks defined as executable predicates instead of explicit metadata
