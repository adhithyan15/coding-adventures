# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-18

### Added

- Implemented HTTP/1 request and response head parsing on top of `http-core`
- Added body framing detection for fixed-length, chunked, bodyless, and until-EOF responses
- Added tests covering CRLF and LF-only input, duplicate headers, bodyless statuses, and malformed heads

### Fixed

- Rewrote the Unix `BUILD` script in explicit POSIX shell form to match the `http-core` CI shell behavior
- Removed quoted extras syntax from the Unix `BUILD` script so the repo build tool's shell wrapper does not break under CI
- Collapsed the Unix `BUILD` flow into line-safe one-command conditionals because the repo build tool executes shell BUILD files one line at a time
- Removed `--no-deps` from the editable dev install so CI still pulls in `pytest` and the other declared `dev` extras
