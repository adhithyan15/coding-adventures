# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-18

### Added

- `StreamReactor` generic over `transport-platform`
- neutral `StreamHandlerResult` for bytes plus close intent
- connection caps and queued-write budget caps
- macOS/BSD `bind_kqueue` convenience constructor
- macOS/BSD end-to-end echo and budget/cap/shutdown tests
