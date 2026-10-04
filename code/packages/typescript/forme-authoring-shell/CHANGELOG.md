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
- Made one captured workspace-level disposal method authoritative for failed
  admission, replacement, and unmount, with fixed redacted cleanup failures.
- Added adversarial browser lifecycle tests for synchronous throws, sparse and
  trapped descriptors, changing getters, hostile result data, duplicate
  actions, and stale settlements while retaining the FM09 coverage thresholds.
