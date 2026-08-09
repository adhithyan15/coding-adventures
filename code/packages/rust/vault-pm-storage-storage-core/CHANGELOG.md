# Changelog

All notable changes to this package are documented here.

## [0.1.0] - 2026-08-09

### Added

- Persistent opaque locator binding over `storage-core` conditional create.
- Hex bucket/namespace and object/key mappings.
- Immutable put, exact reads/stat, stable pagination, and revision-checked
  deletion.
- Shared VLT-PM02 conformance over in-memory and filesystem backends.

### Security

- Storage-core errors are pattern-matched into closed vault errors without
  formatting backend-controlled fields or messages.
- Bodies, locators, identifiers, revisions, paths, and backend values remain
  absent from adapter diagnostics.
- Conditional-create races are re-read and classified as replay or corruption.
