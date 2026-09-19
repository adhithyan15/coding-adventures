# Changelog

## 0.2.0 — 2026-09-19

### Added

- Typed `Stream<RenderedPage>` plus named `Stream<Asset>` fan-in.
- Deterministic SHA-256 asset filenames, placeholder rewriting, static file
  writes, `DeployAssetEntry` records, and complete artifact build identities.
- A normalized `publicPathPrefix` for project-site deployments such as GitHub
  Pages, with per-segment URL encoding.
- Collision, unresolved-reference, malformed-path, byte-length, and
  cancellation diagnostics before or during materialization.
- Explicit side-effect replay from a validated `DeployArtifact`, including
  portable-path and byte validation plus fresh-process output restoration.
- Symlink-safe canonical containment and exclusive temporary-file publication
  for both normal materialization and replay.
