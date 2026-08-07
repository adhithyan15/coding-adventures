# Changelog

All notable changes to the C `vault-policy` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-13

### Added

- Initial pure-ISO C17 port of the Rust `vault-policy` crate (VLT06) — a
  pluggable authorization policy engine.
- `SimpleRbacEngine` (`vp_rbac_new` / `vp_rbac_assign_role` / `vp_rbac_grant` /
  `vp_rbac_summary`): a role × (action, resource-pattern) table with exact and
  `"*"` wildcard matching, and a count-only `VpRbacSummary` of table shape.
- Composable decorators `vp_all_of`, `vp_any_of`, `vp_require_factor`, and
  `vp_time_bound`, each taking ownership of their inner engine(s); a recursive
  `vp_engine_decide` interpreter and `vp_engine_free` teardown over the engine
  tree modelled as a tagged union.
- `VpDecision` (allow / deny with a static reason), the fixed `vp_reason_*`
  reason table (denials never carry attacker bytes), and `VpDecisionRecord` /
  `vp_decide_with_record` for compact owned audit records.
- 75 checks mirroring the Rust crate's own unit tests, run under every available
  C compiler via the shared `iso-harness`; the suite also passes clean under
  AddressSanitizer + UndefinedBehaviorSanitizer.
