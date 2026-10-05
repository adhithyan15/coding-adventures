# Changelog

## Unreleased

## 1.0.0 — 2026-10-05

- Join the aligned stable Forme `1.0.0` package line targeting kernel API v2; see the [`forme-types` migration guide](../forme-types/MIGRATION-v2.md) for the breaking `RenderedPage` provenance and stage/plugin version changes.

### Added

- A deterministic `Stream<ContentNode> -> Stream<Asset>` collector that loads
  each unique resolved filesystem reference once.
- Canonical-root and asset `realpath` containment, including explicit symlink
  escape rejection and regular-file enforcement.
- Binary revision hashing, MIME signature/extension detection, defensive byte
  copies, cancellation, and source/identity/role collision diagnostics.
