# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

### Added

- `State.prolog_flags` extension slot for branch-local Prolog runtime flag
  overlays.
- preservation of the Prolog flag extension slot across equality,
  disequality, and fresh-variable goals.

## [0.4.0] - 2026-04-22

### Added

- `State.fd_store` extension slot for higher-level finite-domain constraint
  stores
- preservation of the finite-domain extension slot across equality,
  disequality, and fresh-variable goals
- tests covering preservation of both runtime extension slots

## [0.3.0] - 2026-04-21

### Added

- `State.database` extension slot for higher-level solver layers that need
  branch-local runtime state
- preservation of the extension slot across equality, disequality, and
  fresh-variable goals
- tests covering extension-state preservation

## [0.2.0] - 2026-04-18

### Added

- `Disequality` constraints stored directly on search states
- `neq(...)` for delayed disequality checks
- constraint-aware `eq(...)` that revalidates stored disequalities after unification
- pytest coverage for delayed constraint storage, violation, and satisfaction

## [0.1.0] - 2026-04-18

### Added

- logic term types: `Atom`, `Number`, `String`, `LogicVar`, and `Compound`
- `Substitution` with walking, extension, and reification support
- unification with occurs-check enabled by default
- goal combinators: `eq`, `succeed`, `fail`, `conj`, `disj`, and `fresh`
- search runners: `run`, `run_all`, and `run_n`
- canonical list construction helper for Prolog-style lists
- pytest coverage for term construction, unification, substitution walking, and search
