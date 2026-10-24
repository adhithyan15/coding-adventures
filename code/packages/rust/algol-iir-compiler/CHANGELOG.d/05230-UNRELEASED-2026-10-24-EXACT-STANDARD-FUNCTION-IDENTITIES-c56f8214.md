## 0.523.0 - 2026-10-24

### Added

- Fold the exact built-in identities `sin(0)`, `arctan(0)`, `cos(0)`, `exp(0)`, and `ln(1)` as leaves of checked static integer reductions while keeping non-identity, dynamic, and overridden calls conservative.
