# Changelog

All notable changes to this package will be documented in this file.

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
