# Changelog

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
