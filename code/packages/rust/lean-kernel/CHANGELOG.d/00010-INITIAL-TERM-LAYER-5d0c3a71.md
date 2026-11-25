## [0.1.0] - 2026-11-23

### Added

- New crate `lean-kernel` (VER11a section 4.2, KRN-002 slice 1: the term layer, no type checker yet).
- `Name`: hierarchical names with string and numeric components.
- `Level`: `zero`/`succ`/`max`/`imax`/`param`, parameter instantiation, `normalize`, `is_equiv`, `is_geq`, `levels_equiv`.
- `ExprStore`/`Expr`: hash-consed expressions (bvar, fvar, sort, const, app, lam, pi, let, lit, proj) with cached hash, loose-bvar range, has-fvar and has-level-param; binder names and info are equality-blind.
- Operations: `instantiate`, `instantiate_rev`, `instantiate1`, `abstract_fvars`, `lift_loose_bvars`, `lower_loose_bvars`, `instantiate_level_params`, `has_loose_bvar`, `get_app_fn`, `get_app_args`.
- Tests: semantic level checks over parameter grids; randomised comparison of de Bruijn operations with naive references.

### Fixed

- De Bruijn index arithmetic is checked: `MAX_BVAR_INDEX` (`u32::MAX / 2`) caps indices (`try_bvar` returns `BVarIndexTooLarge`, `bvar` panics), and shifts panic on overflow instead of wrapping or saturating.
- Hardening: store ids are 64-bit and node/length conversions are checked (`checked_u32`); `FVarId` fields are private and `fvar` refuses ids the store did not issue (`issue_fvar_id_at` for importers); `Level::instantiate` and `instantiate_level_params` return `ArityMismatch` on length mismatch; `MAX_LEVEL_OFFSET` (2^16) with `try_offset`/`try_of_nat`; iterative `Drop` for `Level` and `Name`.
