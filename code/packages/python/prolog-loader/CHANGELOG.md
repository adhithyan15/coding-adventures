# Changelog

## Unreleased

- preserve source query variables across `goal_expansion/2` rewrites
- rewrite module-qualified callable arguments inside `findall/3`, `bagof/3`,
  `setof/3`, and `forall/2`
- expose `rewrite_loaded_prolog_query(...)` for ad-hoc queries that need a
  linked project's module/import context
- adapt Prolog `->/2` and `(If -> Then ; Else)` control constructs into the
  executable logic builtin layer
- expand builtin adaptation for truth/failure/cut, arithmetic, collections,
  `forall/2`, and `copy_term/2`

## 0.1.0

- add `LoadedPrologSource` as a shared loader result over dialect parser outputs
- add `load_iso_prolog_source(...)` and `load_swi_prolog_source(...)`
- add explicit `run_initialization_goals(...)` execution with ordered
  `initialization/1` handling
- support optional goal adaptation so parsed initialization goals can be mapped
  into richer runtime or builtin goals before execution
- add `adapt_prolog_goal(...)` as a shared builtin adapter for parsed Prolog
  goals
- add `run_prolog_initialization_goals(...)` so loader callers can execute
  `call/1`, `dynamic/1`, `assertz/1`, `predicate_property/2`, and related
  builtins without writing custom Python adapters
- add `phrase/2` and `phrase/3` builtin adaptation for DCG-backed grammar calls
- add structured module/import metadata on loaded sources
- add multi-source project linking with namespace-aware `module/2` and
  `use_module/1,2` resolution
- add loader-time rewriting for explicit `module:goal` qualification, including
  linked queries, initialization goals, and common meta-goal wrappers
- add file-backed SWI project loading with recursive `consult/1`,
  `ensure_loaded/1`, and relative `use_module/1,2` resolution
- add `include/1` source splicing for file-backed SWI loader flows
- add pluggable `SourceResolver` hooks so callers can resolve `library(...)`
  and other custom source references during dependency loading
- add explicit `term_expansion/2` and `goal_expansion/2` load-time rewriting
- add `PrologExpansionError` for invalid or non-converging loader expansions
