# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-18

### Added

- Implemented shared `Header`, `HttpVersion`, `BodyKind`, `RequestHead`, and `ResponseHead` types
- Added helper functions for case-insensitive header lookup plus `Content-Length` and `Content-Type`
- Added unit tests covering version parsing, header lookup, content helpers, and semantic head helpers

### Fixed

- Rewrote the Unix `BUILD` script in explicit POSIX shell form to avoid shell parsing differences in CI
- Removed quoted extras syntax from the Unix `BUILD` script so the repo build tool's shell wrapper does not truncate the command
- Collapsed the Unix `BUILD` flow into line-safe one-command conditionals because the repo build tool executes shell BUILD files one line at a time
- Removed `--no-deps` from the editable dev install so CI still pulls in `pytest` and the other declared `dev` extras
