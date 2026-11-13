# Changelog

## Unreleased

### Fixed

- Restructured `metadata_is_link` so each cfg arm is a single expression,
  satisfying clippy 1.100 `needless_bool` on non-Windows builds; behavior
  unchanged on all platforms.

## 0.1.0 - 2026-08-03

- Add validated install plans for launchd, systemd user services, and Windows
  Task Scheduler.
- Publish service definitions through a synchronized unique sibling and atomic
  hard-link claim without overwriting an existing definition.
- Make byte-identical installations retryable after registration failure.
- Reject linked inputs, writable Unix service directories, and non-regular
  supervisor executables before publication.
- Execute absolute native supervisor commands directly through an injectable,
  shell-free command boundary.
