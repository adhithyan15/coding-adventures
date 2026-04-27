# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-17

### Added

- Deterministic LaunchAgent plist rendering for portable job specs
- Native trigger mapping for `StartInterval`, `StartCalendarInterval`, and
  `RunAtLoad`
- Explicit unsupported-case errors for one-shot LaunchAgent schedules, boot
  triggers, interval anchors, and stdin payloads
- Install-plan generation with plist write targets plus `launchctl` commands
- Renamed the package to `macos-job-backend-launchd-files` for clearer OS
  scoping
