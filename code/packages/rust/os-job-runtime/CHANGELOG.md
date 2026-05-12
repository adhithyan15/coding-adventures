# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-17

### Added

- `NativeJobRuntime` for current-OS or explicit backend selection
- Delegation to the `launchd`, `systemd --user`, and Windows XML backends
- Explicit in-process fallback backend selection and deterministic install-plan
  generation for tests, development sandboxes, and constrained hosts
- Backend-kind discovery helpers and integration tests covering all three
  supported native scheduler families plus the in-process fallback
- Strict portability validation that rejects jobs outside the current
  macOS/Linux/Windows portable subset before backend planning
- Read-side portability issue queries and backend/field summaries for bounded
  D18E inspection tools
- Backend-level portability status rows for D18E/tool surfaces that need one
  supported-or-blocked result per scheduler family
- Runtime backend summaries for selected backend, native/fallback catalog shape,
  and active portability target
- Renamed the package to `os-job-runtime` to make the OS scheduling layer easier
  to discover
