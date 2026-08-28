# Changelog

## Unreleased

### Added

- Typed `Stream<RenderedPage>` plus named `Stream<Asset>` fan-in.
- Deterministic SHA-256 asset filenames, placeholder rewriting, static file
  writes, `DeployAssetEntry` records, and complete artifact build identities.
- A normalized `publicPathPrefix` for project-site deployments such as GitHub
  Pages, with per-segment URL encoding.
- Collision, unresolved-reference, malformed-path, byte-length, and
  cancellation diagnostics before or during materialization.
