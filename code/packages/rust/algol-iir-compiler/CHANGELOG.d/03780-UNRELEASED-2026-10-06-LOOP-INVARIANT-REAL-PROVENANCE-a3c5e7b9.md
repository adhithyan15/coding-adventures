## [0.378.0] - UNRELEASED

### Added
- Preserve runtime-real formatter provenance across `for` loops for
  caller-frame real locals whose values the loop body leaves invariant. The
  controlled variable, changed locals, and shared alias paths remain excluded.
