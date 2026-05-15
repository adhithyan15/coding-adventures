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
- Added an end-to-end sandbox-plan proof that lowers the Weather Agent
  capability manifest into OS-specific defense-in-depth primitives for Linux,
  macOS, Windows, FreeBSD, OpenBSD, and the portable host broker.
- Added a statically linked generated operation-side HTTP client so live
  Weather.gov operations refuse any domain that is not declared in
  `required_capabilities.json`, without parsing the JSON at runtime.
