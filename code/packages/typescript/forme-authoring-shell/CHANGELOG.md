# Changelog

All notable changes to `@coding-adventures/forme-authoring-shell` are recorded
here.

## 0.1.0 — 2026-10-04

### Added

- Added the capability-free FM09 first-run React shell over the existing
  authoring core, editor, exact preview, and reviewed publication boundaries.
- Added bounded host and workspace admission, closed theme and target review,
  loopback-only preview URLs, fixed failure messages, and explicit two-step
  publication confirmation.
- Added abortable loading and creation plus one shared idempotent workspace
  retirement boundary, including late completion after unmount.
- Hardened every host call behind a Promise boundary, admitted one action
  synchronously, isolated raw sessions behind a validated snapshot facade, and
  validated preview/publication attribution before rendering results.
- Validated canonical base64 SHA-256 publication identities with the authoring
  core's shared validator and treated malformed or rejected publisher
  settlements as indeterminate rather than safe-to-retry failures.
- Made one captured workspace-level disposal method authoritative for failed
  admission, replacement, and unmount, serialized replacement behind its
  successful settlement, and poisoned interaction after missing, failed, or
  stale retirement.
- Poisoned the workspace after a committed session mutation cannot be safely
  resnapshotted, preventing a stale facade from reporting unsaved state or
  accepting later mutations before reload.
- Snapshotted array length from one own data descriptor before bounded
  iteration so proxy-controlled reads cannot expand admission work.
- Added adversarial browser lifecycle tests for synchronous throws, sparse and
  trapped descriptors, changing getters, hostile result data, duplicate
  actions, real publisher integration, serialized replacement, and stale
  retirement while retaining the FM09 coverage thresholds.
