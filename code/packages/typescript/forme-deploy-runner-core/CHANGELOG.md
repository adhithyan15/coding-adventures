# Changelog — @coding-adventures/forme-deploy-runner-core

## 0.1.0 — 2026-09-20

Initial FM-B044 release of the capability-free Forme deploy-runner core.

### Added

- Strict untrusted-manifest parsing with canonical serialization, portable
  output-path validation, exact count/size checks, and prefix-collision
  rejection.
- Stable complete-set planning across current and previous manifests with
  explicit create, update, skip, and previous-owned delete actions.
- Content-store preflight that verifies availability, byte length, and SHA-256
  with deduplicated bounded retention and cooperative cancellation.
- A manifest-bound verified-content reader that parses once and returns the
  exact trusted plain byte snapshot an adapter is permitted to publish.
- Byte-deterministic dry-run reports with sorted paths, fixed field order,
  deterministic timestamps, and zero write metrics.

### Security

- The package declares no capabilities and opens no I/O boundary.
- A previous manifest can authorize deletion only for its validated paths.
- Prototype sink names, control characters, absolute paths, traversal,
  backslashes, drive prefixes, reserved device names, and file/directory prefix
  collisions are rejected before planning.
- Manifest/file/total-size limits, strict URL/route/date/MIME metadata checks,
  immutable bound plans, and exact-snapshot verification close resource,
  header-injection, path-mutation, and changing-store races.
- Canonical base64 digests remain opaque keys; a reviewed base64url conversion
  supplies one safe filename segment for future directory and bundle stores.
