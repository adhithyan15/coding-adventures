# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Added

- `RequestTrackerSummary` and `RequestTracker::summary()` for compact
  diagnostics over pending Serial API requests, including callback/response
  waits, per-function counts, oldest sent time, and next timeout.

## [0.1.0] - 2026-05-06

### Added

- Serial API function id, request/response/callback classification, controller
  capability, Memory Get ID, and request tracking primitives.
- Bootstrap request builders for version, init-data, controller-capability, and
  Memory Get ID reads plus typed Serial API version parsing.
- Callback correlation and timeout expiry helpers for future controller loops.
- Serial API Get Init Data node inventory parsing and Application Command
  Handler envelopes for command-class routing.
- Application Command Handler-to-command-class frame projection plus SendData
  request, response, callback, and transmit-option primitives.
- SendData transaction state machine for accepted responses, terminal callback
  outcomes, callback-id mismatches, and timeout expiry.
- Deterministic bootstrap request plan for controller startup using version,
  Memory Get ID, controller capabilities, and init-data requests.
