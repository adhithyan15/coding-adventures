# Changelog

## 0.1.0

- add `LoadedPrologSource` as a shared loader result over dialect parser outputs
- add `load_iso_prolog_source(...)` and `load_swi_prolog_source(...)`
- add explicit `run_initialization_goals(...)` execution with ordered
  `initialization/1` handling
- support optional goal adaptation so parsed initialization goals can be mapped
  into richer runtime or builtin goals before execution
