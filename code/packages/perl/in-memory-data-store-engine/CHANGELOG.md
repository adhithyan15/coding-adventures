# Changelog

## [Unreleased]

### Fixed

- `BUILD_windows` passes dependency library paths with `-I` instead of `set PERL5LIB=...`, which MSYS perl on CI splits on `:` and so never found.

## 0.1.0 - 2026-07-16

- Add the full in-memory data store execution engine for Perl.
- Cover typed collections, sorted sets, HyperLogLog, TTLs, logical databases,
  protocol errors, deterministic ordering, and binary-safe values.
