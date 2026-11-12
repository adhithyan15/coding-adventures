# Changelog

All notable changes to Unix Tools will be documented in this file.

## [Unreleased]

### Fixed

- `id` tests: the "non-empty group list" assertion now runs on POSIX only.
  On the Windows runner, Git for Windows' `id` printed no groups for the CI
  account, so the test failed the first time this program was rebuilt on
  Windows. The group list is still checked to be a list everywhere.
- `mix.lock`: jason 1.4.4 -> 1.4.5, aligned with every other lock in the repo.

## [1.0.0] - 2026-03-21

### Added

- Initial release as `unix_tools`, consolidating individual Unix tool programs into a single package.
- `UnixTools.Pwd` — reimplementation of POSIX `pwd` with `-L` (logical) and `-P` (physical) modes.
- Full CLI Builder integration via `pwd.json` spec file.
- Comprehensive test suite covering flag parsing, help/version output, error handling, and business logic.

### Changed

- Restructured from standalone `pwd` package into the `unix_tools` umbrella.
- Module renamed from `Pwd.CLI` to `UnixTools.Pwd`.
