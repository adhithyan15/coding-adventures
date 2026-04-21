# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-20

### Added

- Added a stateful Mini Redis worker with string, hash, counter, and `SELECT`
  support.
- Added support for the shared generic job-protocol JSON-line envelope, with
  opaque TCP byte jobs and opaque write frames carried as payload fields.
- Added Python-owned RESP framing, per-stream buffering, pipelined command
  handling, and stream-local `SELECT` session state.
- Added unit tests for command correctness, protocol round trips, malformed
  RESP input handling, wrong-type errors, job queueing, and CLI startup.
