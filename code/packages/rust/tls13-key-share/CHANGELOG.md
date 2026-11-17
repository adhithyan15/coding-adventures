# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Changed

- Admit and encode the exact empty ClientHello key-share vector permitted to
  request HelloRetryRequest group selection, while keeping direct ServerHello
  selection bound to a nonempty retained offer. The prior public empty-list
  error category remains available for source compatibility but is no longer
  emitted.

## [0.1.0] - 2026-10-30

### Added

- Allocation-free TLS 1.3 ClientHello key-share validation, framing, and
  decoding under repository-owned named-group policy.
- Exact ServerHello key-share decoding bound to the retained client offer.
- Stable payload-free failures, redacted evidence, caller-owned output, and
  empty required capabilities.
