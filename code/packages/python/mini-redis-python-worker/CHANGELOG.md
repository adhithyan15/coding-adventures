# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-20

### Added

- Added a stateful Mini Redis worker with string, hash, counter, and `SELECT`
  support.
- Added support for the shared generic job-protocol JSON-line envelope, with
  Redis command arguments and engine responses carried as payload fields.
- Added unit tests for command correctness, protocol round trips, malformed
  input handling, wrong-type errors, and CLI startup.
- Refined the worker boundary to mirror the WASM Mini Redis adapter: Rust owns
  RESP framing and per-connection selected database state, while Python receives
  command-frame jobs and returns engine-response payloads.
