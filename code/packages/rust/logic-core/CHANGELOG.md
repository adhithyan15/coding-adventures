# Changelog

All notable changes to this project will be documented in this file.

## [0.2.1] - 2026-07-14

### Changed

- **`Number::Exact` `Display` now follows the NX-2 rendering policy.** An exact value with
  **≤ 17 significant digits** (everything an `f64` can represent) renders through its `f64`
  canonical form, so its printed string is byte-for-byte identical to the value's old `Float`
  rendering — trailing-zero normalization (`20.180` → `20.18`), extreme-magnitude formatting, and
  so on all match. A value with **more than 17 significant digits** (π to 39 places) prints every
  exact digit it carries. This makes exactness a strict *superset* of the old output: nothing that
  already fit an `f64` changes how it looks, and only genuinely higher-precision values gain digits.
  Depends on `bignum-core` ≥ 0.5.0 for `BigDecimal::significant_digits`. (Nothing produced `Exact`
  before NX-2, so this only affects values created by the exact-literal lowering that lands with it.)

## [0.2.0] - 2026-07-14

### Added

- `Number::Exact(BigDecimal)` — a third numeric variant holding an unbounded exact
  decimal, so a written decimal literal (`3.14159…`, `6.022e23`) keeps every digit
  instead of being truncated to `f64` at parse time (see
  `code/specs/ADJ-EXACT-NUMBERS.md`). This is NX-1 of the exact-numbers arc; nothing
  *produces* `Exact` yet (literals still lower via `f64` until NX-2), so this release
  is a pure, behavior-preserving type widening.
- `Number::to_f64_lossy()` — the single, greppable, sanctioned way to drop a `Number`
  to `f64` (the "labeled lossy export" boundary). `Exact` rounds via `BigDecimal::to_f64`.
- New dependency on `bignum-core` (for `BigDecimal`).

### Changed

- **Breaking:** `Number` no longer derives `Copy` — `Exact` wraps a heap-backed
  `BigDecimal`, so `Number` now moves/clones like `Term`. Callers that copied a
  `Number` by value must move or `.clone()`.
- `Number`'s `Display` prints an `Exact` value with all of its digits (no truncation).
- Equality stays **variant-distinct** (Prolog tradition): `Int(1)`, `Float(1.0)`, and
  `Exact(1.0)` are three different ground terms; numeric reconciliation remains a
  compute-layer concern.

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
