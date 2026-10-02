# Changelog

## Unreleased

### Added

- Verify `RenderedPage.usedIslands` against exact island-module asset uses,
  emit fingerprinted external module tags, and preserve those IDs in deploy
  routes.
- Prune unreferenced `script` assets while preserving the zero-JavaScript path
  for static pages.

### Security

- Reject missing, duplicate, non-script, non-JavaScript-MIME, or usage-list
  mismatches before materializing an interactive page. Recompute reviewed
  script SHA-256 bindings, snapshot bounded page usage, and reject exact or
  portable case/Unicode-normalization output-path collisions before writes.

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
