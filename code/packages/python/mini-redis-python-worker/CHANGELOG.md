# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-20

### Added

- Added a stateful Mini Redis worker with string, hash, counter, and
  connection-local `SELECT` support.
- Added support for the shared generic job-protocol JSON-line envelope, with
  Redis command arguments and RESP replies carried as payload fields.
- Added unit tests for command correctness, protocol round trips, malformed
  input handling, wrong-type errors, and CLI startup.
