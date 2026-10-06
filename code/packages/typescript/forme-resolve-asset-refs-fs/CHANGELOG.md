# Changelog

## Unreleased

## 1.0.0 — 2026-10-05

- Join the aligned stable Forme `1.0.0` package line targeting kernel API v2; see the [`forme-types` migration guide](../forme-types/MIGRATION-v2.md) for the breaking `RenderedPage` provenance and stage/plugin version changes.

### Added

- A stream transform that discovers local Document AST images and emits
  normalized `AssetRef` values with one identity per source path.
- Root-escape protection, optional UUIDv7 identity sidecars, duplicate-path
  deduplication, identity-collision diagnostics, and cancellation checks.
- Query strings and fragments preserved separately from filesystem identity so
  renderers and emitters can retain authored URL semantics.
