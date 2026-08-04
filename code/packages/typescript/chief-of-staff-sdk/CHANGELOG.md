# Changelog

All notable changes to this package will be documented in this file.

## [0.2.0] - 2026-08-03

### Added

- Level 2 `defineAgent` registration for one-file TypeScript agents.
- Injected `SimpleAgentRuntime` with receive, handler, publish, acknowledge
  ordering and fail-closed UTF-8/output validation.

## [0.1.0] - 2026-08-03

### Added

- Strict line-delimited JSON-RPC transport for the D18 host protocol.
- Level 3 `channel_read`, `channel_write`, and `channel_ack` APIs.
- Lossless integer and binary-payload decoding with fail-closed validation.
