# Changelog

All notable changes to this package will be documented in this file.

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
