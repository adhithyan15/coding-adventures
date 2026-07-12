# Changelog

All notable changes to the C++ `read-write-separation` package are documented
here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-12

### Added

- Initial header-only pure-ISO C++17 port of the Rust `read-write-separation`
  crate (namespace `ca::read_write_separation`).
- A value-semantic `Capability` with a fluent `with_flavor` / `with_trust` /
  `with_justification` builder and `identifier()`; the `Flavor` / `Trust`
  scoped enums with `to_string`.
- `classify_capability` (flavor/trust inference, loopback and
  `package:`-internal target detection), `summarize_manifest` (`Summary` with 13
  counts plus `has_rws_risk` / `has_same_resource_overlap` / `is_empty`), and
  `validate_manifest` returning `std::optional<Violation>` (empty == valid) in
  place of the Rust `Result<(), _>`.
- Glob-prefix overlap detection and value-based `push_unique` deduplication,
  matching the Rust crate.
- 51 checks against the Rust crate's own test vectors, run under every available
  C++ compiler via the shared `iso-harness`.
