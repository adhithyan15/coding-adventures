# Changelog

## [Unreleased]

### Added

- New crate, split out of `coding_adventures_hash_functions` 0.1.0 (WX-P1a2, DT17a
  section 7): `avalanche_score` and `distribution_test`, moved unchanged from
  `hash-functions/src/analysis.rs`. Same signatures and behaviour; only the crate
  path changes (`coding_adventures_hash_analysis::...`).
- `required_capabilities.json` declaring `rand:read:*` (OS randomness for input
  sampling), checked by the capability analyzer in `BUILD`.
- Tests beyond the three that moved: a real hash against a seeded source, a strong
  hash close to 0.5, the OS-backed entry point, exact output-bit accounting, an
  even spread scoring 0, and every `assert!` precondition (`should_panic`).
- The README usage example runs as a doctest (`#[cfg(doctest)]` include), as in `hash-functions`.
