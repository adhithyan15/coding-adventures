# Changelog

## Unreleased

### Added

- A stream transform that discovers local Document AST images and emits
  normalized `AssetRef` values with one identity per source path.
- Root-escape protection, optional UUIDv7 identity sidecars, duplicate-path
  deduplication, identity-collision diagnostics, and cancellation checks.
- Query strings and fragments preserved separately from filesystem identity so
  renderers and emitters can retain authored URL semantics.
