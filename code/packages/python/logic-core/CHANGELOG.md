# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-18

### Added

- logic term types: `Atom`, `Number`, `String`, `LogicVar`, and `Compound`
- `Substitution` with walking, extension, and reification support
- unification with occurs-check enabled by default
- goal combinators: `eq`, `succeed`, `fail`, `conj`, `disj`, and `fresh`
- search runners: `run`, `run_all`, and `run_n`
- canonical list construction helper for Prolog-style lists
- pytest coverage for term construction, unification, substitution walking, and search
