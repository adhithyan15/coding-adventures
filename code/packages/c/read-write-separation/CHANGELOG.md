# Changelog

All notable changes to the C `read-write-separation` package are documented
here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C17 port of the Rust `read-write-separation` crate.
- `RwsCapability` value type with builder-style setters
  (`rws_capability_init` / `set_flavor` / `set_trust` / `set_justification` /
  `release` / `identifier`) and the `RwsFlavor` / `RwsTrust` enums with
  `rws_flavor_str` / `rws_trust_str`.
- `rws_classify` (flavor/trust inference from category/action, loopback and
  `package:`-internal target detection), `rws_summarize` (`RwsSummary` with 13
  counts plus `has_rws_risk` / `has_same_resource_overlap` / `is_empty`), and
  `rws_validate` (untrusted-input + external-actuation and glob-overlap
  detection), returning `RwsStatus` in place of the Rust `Result<(), _>`.
- `RwsViolation` borrows pointers into the analyzed array plus an owned message;
  the borrowed-pointer list dedups by capability value, matching the Rust
  `push_unique`.
- 88 checks against the Rust crate's own test vectors, run under every available
  C compiler via the shared `iso-harness`.
