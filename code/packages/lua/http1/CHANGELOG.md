# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-18

### Added

- Implemented HTTP/1 request and response head parsing on top of `http-core`
- Added body framing detection for fixed-length, chunked, bodyless, and until-EOF responses
- Added tests covering CRLF and LF-only input, duplicate headers, bodyless statuses, and malformed heads

### Fixed

- Pinned the rockspec source to an immutable HTTPS git ref for reproducible installs
- Relaxed error-message assertions so tests accept the parser's detailed invalid-header and invalid-length messages
- Relaxed the malformed-head assertions again so they accept the source-location prefix that Lua `pcall` adds around parser errors in CI
