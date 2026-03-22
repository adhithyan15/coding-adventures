# Changelog

All notable changes to Unix Tools will be documented in this file.

## [1.0.0] - 2026-03-21

### Added

- Initial release as `unix_tools`, consolidating individual Unix tool programs into a single package.
- `UnixTools.Pwd` — reimplementation of POSIX `pwd` with `-L` (logical) and `-P` (physical) modes.
- Full CLI Builder integration via `pwd.json` spec file.
- Comprehensive test suite covering flag parsing, help/version output, error handling, and business logic.

### Changed

- Restructured from standalone `pwd` package into the `unix_tools` umbrella.
- Module renamed from `Pwd.CLI` to `UnixTools.Pwd`.
