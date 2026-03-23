# Changelog

All notable changes to this project will be documented in this file.

## [1.0.0] - 2026-03-21

### Added

- Initial release as `unix-tools`, restructured from the standalone `pwd` package.
- `pwd` tool: prints the absolute pathname of the current working directory.
  - Supports `-L`/`--logical` (default) and `-P`/`--physical` modes.
  - Uses CLI Builder for declarative argument parsing via `pwd.json`.
