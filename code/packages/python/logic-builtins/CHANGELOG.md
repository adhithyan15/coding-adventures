# Changelog

All notable changes to this package will be documented in this file.

## [0.8.0] - 2026-04-21

### Added

- dynamic database predicates: `dynamico`, `assertao`, `assertzo`,
  `retracto`, `retractallo`, and `abolisho`
- predicate metadata now observes branch-local dynamic declarations and
  reports `dynamic` properties
- `clauseo` now sees runtime dynamic clauses from the active search state
- tests covering assertion order, rollback across branches, retraction
  bindings, retract-all, abolish, static-predicate protection, dynamic source
  clauses, metadata, and clause introspection

## [0.7.0] - 2026-04-21

### Added

- `calltermo(term_goal)` for executing reified Prolog-shaped goal terms
- standard term-order predicates: `compare_termo`, `termo_lto`,
  `termo_leqo`, `termo_gto`, and `termo_geqo`
- predicate metadata predicates: `current_predicateo` and
  `predicate_propertyo`
- tests proving clause-body round trips, non-binding term comparisons, and
  source/builtin predicate metadata

## [0.6.0] - 2026-04-21

### Added

- `clauseo(head, body)` for Prolog-style clause introspection from inside logic queries
- support for relation-call head arguments, source-order clause enumeration, fact bodies as `true`, rule body term encoding, and standardize-apart behavior
- tests covering head/body filtering, instantiated rule bodies, returned variable freshness, and host-only body skipping

## [0.5.0] - 2026-04-20

### Added

- term metaprogramming predicates: `univo`, `copytermo`, `same_termo`, `atomico`, and `callableo`
- construction-mode `functoro` for atoms and compounds with fresh argument variables
- tests covering Prolog-style term decomposition, construction, variable-refreshing copies, strict identity, and callable/atomic classification

## [0.4.0] - 2026-04-20

### Added

- advanced control predicates: `trueo`, `failo`, `iftheno`, `ifthenelseo`, and `forallo`
- tests covering committed-condition behavior, then-branch backtracking, else-state isolation, and forall binding discipline
- documentation explaining why real Prolog cut is deferred until the solver can prune scoped choicepoints

## [0.3.0] - 2026-04-20

### Added

- collection predicates: `findallo`, `bagofo`, and `setofo`
- deterministic term sorting and duplicate removal for first-pass `setofo`
- tests and examples showing collectors with relation search, arithmetic, and control builtins

## [0.2.0] - 2026-04-20

### Added

- arithmetic expression constructors: `add`, `sub`, `mul`, `div`, `floordiv`, `mod`, and `neg`
- `iso(result, expression)` as the library spelling of Prolog's evaluative `is/2`
- numeric comparison predicates: `numeqo`, `numneqo`, `lto`, `leqo`, `gto`, and `geqo`
- tests and examples showing arithmetic composition with relation search and control builtins

## [0.1.0] - 2026-04-20

### Added

- Prolog-inspired control predicates: `callo`, `onceo`, and `noto`
- term state/type predicates: `groundo`, `varo`, `nonvaro`, `atomo`, `numbero`, `stringo`, and `compoundo`
- first inspection-mode structural predicates: `functoro` and `argo`
