# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-20

### Added

- Added a stateful Mini Redis worker with string, hash, counter, and
  connection-local `SELECT` support.
- Added a JSON-line worker protocol that accepts hex-encoded command arguments
  and returns hex-encoded RESP replies.
- Added unit tests for command correctness, protocol round trips, malformed
  input handling, wrong-type errors, and CLI startup.
