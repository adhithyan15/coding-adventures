# Changelog

All notable changes to this package will be documented in this file.

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
