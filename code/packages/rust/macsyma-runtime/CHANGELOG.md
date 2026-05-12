# Changelog

## [0.1.1] - 2026-05-12

### Added

- Wired `linsolve` / `Solve(List(...), List(...))` to the Rust `cas-solve`
  exact linear-system solver.
- Added runtime coverage for integer systems, rational systems, and non-linear
  fallback.

## [0.1.0] - 2026-05-08

### Added

- Initial Rust MACSYMA runtime session facade.
- Source-to-IR compilation, symbolic VM evaluation, display/suppress result metadata, and in-memory input/output history.
