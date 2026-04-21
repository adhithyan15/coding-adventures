# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-20

### Added

- Added a prototype `PythonMiniRedisServer` built on `tcp-runtime`.
- Added per-connection RESP buffering and command delegation to a Python worker
  process.
- Added a JSON-line worker protocol with hex-encoded request arguments and
  RESP response bytes.
- Added Rust integration tests that start a TCP listener, call the Python
  worker, and validate Redis replies over a real socket.
