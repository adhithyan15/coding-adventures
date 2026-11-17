# Changelog

All notable changes to this package will be documented in this file.

## [0.2.0] - 2026-11-17

### Added

- Final ServerHello key-share decoding bound directly to the completed retry
  transition's exact retained second-ClientHello offer.
- Non-copyable continuity evidence retaining both the borrowed transition and
  decoded final server share, with stable payload-free failure categories.

## [0.1.0] - 2026-11-17

### Added

- Exact two-byte HelloRetryRequest key-share group decoding under the existing
  data-driven named-group policy.
- Non-copyable evidence binding the selected group to the first
  `supported_groups` list, its absence from the first key-share offer, and the
  sole second-ClientHello share.
- Stable payload-free failures and redacted diagnostics.
