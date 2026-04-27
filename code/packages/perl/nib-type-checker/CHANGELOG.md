# Changelog

## [0.1.0] - 2026-04-18

### Added

- Semantic checking for variables, returns, loops, calls, and boolean
  expressions in Perl's Nib frontend.
- `check_source()` and `check()` helpers that return the shared
  `type-checker-protocol` result shape.
- Coverage for successful programs, assignment mismatches, and parse failures.
