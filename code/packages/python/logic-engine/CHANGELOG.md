# Changelog

All notable changes to this package will be documented in this file.

## [0.10.0] - 2026-04-22

### Added

- preservation of the `State.fd_store` extension slot across engine state-copy
  helpers so finite-domain builtins can compose with dynamic database overlays,
  fresh variables, rules, and relation calls

## [0.9.0] - 2026-04-22

### Added

- `CutExpr` and `cut()` as the library representation of Prolog `!/0`
- internal search-result cut propagation so conjunctions and disjunctions can
  prune scoped choicepoints without changing the public `solve` iterator API
- relation-call cut frames that consume cuts raised inside rule bodies while
  still pruning later clauses for that predicate invocation
- `goal_as_term`/`goal_from_term` support for encoding and lowering cut as
  the atom `!`
- tests proving cut prunes choices before it, keeps choices after it, commits
  rule bodies and later clauses, and stays scoped to the callee relation

## [0.8.0] - 2026-04-21

### Added

- dynamic predicate declarations on `Program`
- branch-local `DynamicDatabase` overlays for runtime database mutation
- runtime helpers for `asserta`, `assertz`, `retract`, `retractall`, and
  `abolish` semantics during active search
- visible-clause and visible-predicate helpers that include dynamic state
- `clause_from_term(...)` for lowering Prolog-shaped clause data back into
  engine clauses
- tests covering dynamic source clauses, branch-local assertion visibility,
  runtime retraction, visible-clause ordering, and clause-term lowering

## [0.7.0] - 2026-04-21

### Added

- `goal_from_term(...)` for lowering Prolog-shaped callable terms back into
  executable engine goals
- support for lowering truth, failure, equality, disequality, conjunction,
  disjunction, relation-call compounds, and zero-arity callable atoms
- tests covering executable round trips and malformed callable-term rejection

## [0.6.0] - 2026-04-21

### Added

- clause and goal term encoders: `clause_body`, `goal_as_term`, and `clause_as_term`
- public `freshen_clause` helper for standardizing clauses apart outside the solver
- tests covering truth bodies for facts, Prolog-shaped goal encoding, unsupported host-only goals, clause term encoding, and freshened variable aliasing

## [0.5.0] - 2026-04-20

### Added

- persistent clause database helpers: `asserta`, `assertz`, `clauses_matching`,
  `retract_first`, `retract_all`, and `abolish`
- tests proving database updates preserve source order, return new `Program`
  values, match heads through unification, and leave original programs unchanged

## [0.4.0] - 2026-04-20

### Added

- `NativeGoalExpr` and `native_goal(...)` for state-aware library predicates
- `solve_from(...)` so native goals can evaluate nested goals from the active search state
- public `reify` re-export for builtins that need to inspect current term values

## [0.3.0] - 2026-04-18

### Added

- `defer(...)` and `DeferredExpr` for solve-time expansion of recursive host-language goal builders
- pytest coverage proving deferred recursive helper goals work over concrete lists

## [0.2.0] - 2026-04-18

### Added

- `neq(...)` goal expressions delegated to `logic-core` disequality constraints
- `all_different(...)` for pairwise-distinct puzzle variables
- end-to-end tests for delayed disequality and a small coloring problem

## [0.1.0] - 2026-04-18

### Added

- relation objects with arity checking
- fact and rule clauses
- immutable programs indexed by relation
- recursive solver with clause standardization-apart
- `solve`, `solve_all`, and `solve_n`
- pytest coverage for facts, rules, recursion, tuple queries, and list relations
