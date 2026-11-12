# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

### Fixed

- No source change; the clippy 1.100 `needless_bool` failure came from the
  `context-store` dependency, fixed there.

## [0.1.0] - 2026-05-18

### Added

- Initial D18A `storage-local-folder` crate.
- `LocalFolderStorageBackend`, the spec-named local-folder adapter over the
  existing STR-FILE filesystem backend.
- Payload-free `LocalFolderStorageBackendSummary` helpers that expose storage
  guarantees without leaking root paths, namespaces, keys, metadata, or bodies.
- Shared `storage-core` backend conformance coverage.
- A `ContextStore` persistence smoke test proving D18A stores can survive a
  backend/process rebuild over the same local folder.
