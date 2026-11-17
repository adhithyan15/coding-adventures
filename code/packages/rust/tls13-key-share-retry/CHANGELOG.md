# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-11-17

### Added

- Exact two-byte HelloRetryRequest key-share group decoding under the existing
  data-driven named-group policy.
- Non-copyable evidence binding the selected group to the first
  `supported_groups` list, its absence from the first key-share offer, and the
  sole second-ClientHello share.
- Stable payload-free failures and redacted diagnostics.
