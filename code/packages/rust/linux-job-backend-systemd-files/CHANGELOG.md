# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-17

### Added

- Deterministic `.service` and `.timer` rendering for `systemd --user`
- Native `OnCalendar` and monotonic timer mapping from portable job triggers
- Per-user login activation support through `WantedBy=default.target`
- Explicit unsupported-case errors for boot scheduling in the user manager,
  interval anchors, and stdin payloads
- Renamed the package to `linux-job-backend-systemd-files` for clearer OS
  scoping
