## [Unreleased]

### Added

- Four sixth computed-name assigned value programs require exact9/1 stdout,
  normal exit0, actual per-program execution and all available-backend checks.
  Saved locals avoid ALGOL result-return semantics. Actual24 cells pass across
  six backends; CLR unavailable/unvalidated. Adversarial missing/nonzero exit
  checks are simulated and excluded from runtime evidence.
- Retain all earlier tests/shared intentional-return stdout helper. Original
  stdout-only review finding is preserved; scoped repaired review PASS.
- Final-source Miri is DEFERRED/PENDING; downstream/current-main gates pending.
