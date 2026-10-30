# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-10-30

### Added

- Allocation-free TLS 1.3 ClientHello key-share validation, framing, and
  decoding under repository-owned named-group policy.
- Exact ServerHello key-share decoding bound to the retained client offer.
- Stable payload-free failures, redacted evidence, caller-owned output, and
  empty required capabilities.
