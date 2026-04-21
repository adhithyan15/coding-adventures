# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-20

### Added

- Added a prototype `PythonMiniRedisServer` built on `tcp-runtime`.
- Added per-connection RESP buffering and command delegation to a Python worker
  process.
- Added integration with the shared `generic-job-protocol` crate so Redis
  command arguments and RESP response bytes are payloads inside reusable
  `JobRequest` / `JobResponse` frames.
- Added Rust integration tests that start a TCP listener, call the Python
  worker, and validate Redis replies over a real socket.
