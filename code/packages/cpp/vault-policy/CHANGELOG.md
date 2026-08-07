# Changelog

All notable changes to the C++ `vault-policy` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-13

### Added

- Initial header-only pure-ISO C++17 port of the Rust `vault-policy` crate
  (VLT06, namespace `ca::vault_policy`) — a pluggable authorization policy
  engine.
- `SimpleRbacEngine` (`assign_role` / `grant` / `summary`): a role × (action,
  resource-pattern) table with exact and `"*"` wildcard matching, plus a
  count-only `SimpleRbacSummary`.
- Composable decorators `AllOf`, `AnyOf`, `RequireFactor`, and `TimeBound`,
  built on an abstract `PolicyEngine` base with virtual `decide`; decorators own
  their inner engines via `std::unique_ptr`.
- `Decision` (allow / deny with `reason()` → `std::optional<Reason>`), the fixed
  `Reason::*` static-literal table, and `PolicyDecisionRecord` /
  `decide_with_record` for compact owned audit records.
- 67 checks mirroring the Rust crate's own unit tests, run under every available
  C++ compiler via the shared `iso-harness`; the suite also passes clean under
  AddressSanitizer + UndefinedBehaviorSanitizer.
