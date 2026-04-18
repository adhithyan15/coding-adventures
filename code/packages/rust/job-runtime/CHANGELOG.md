# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-17

### Added

- `NativeJobRuntime` for current-OS or explicit backend selection
- Delegation to the `launchd`, `systemd --user`, and Windows XML backends
- Backend-kind discovery helpers and integration tests covering all three
  supported native scheduler families
