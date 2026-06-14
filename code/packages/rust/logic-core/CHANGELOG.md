# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-05-10

### Added

- `Term` enum with five variants: `Atom`, `Number`, `Str`, `Var`, `Compound`
- `Number` enum splitting integer (`i64`) and floating-point (`f64`) values
- `LogicVar` with a monotonically increasing global id and optional display name
- `var()` constructor that allocates a fresh `LogicVar` from a process-wide counter
- Convenience constructors: `atom()`, `int()`, `float()`, `string()`, `compound()`, `logic_list()`
- `Substitution` — a copy-on-write map from `LogicVar` id to bound `Term`
- `walk()` — chase variable bindings through a substitution to a root term
- `unify()` — first-order unification with occurs-check, returns a new `Substitution` on success and `None` on failure
- `Display` implementations on all terms producing Prolog-style output
- Inline unit tests and an `tests/test_unification.rs` integration suite
- Doc comments explain the intent of each type, not just the mechanics — the package is the Rust port of the Python `logic-core` (LP00).

### Notes

This crate is the Rust starting point for the logic VM ports tracked by
`LP00..LP18` in `code/specs/`. The Python implementation under
`code/packages/python/logic-core` remains the reference. Subsequent PRs will
add disequality, goals, search, and the run/run_n driver, mirroring the
Python API surface.
