# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

### Added

- Initial umbrella-today end-to-end harness that exercises the Chief of Staff
  actor, D18A store, D18C job, D18D tool, read/write separation, and
  capability-caged file-write path with a deterministic Seattle weather fixture.
- Added a live Weather.gov HTTPS mode through `tls-platform` + `http1`, plus a
  supervisor restart proof that kills and recreates a weather child before the
  pipeline tick.
