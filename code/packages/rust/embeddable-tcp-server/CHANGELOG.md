# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-20

### Added

- Added `EmbeddableTcpServer`, a language-neutral TCP bridge built on
  `tcp-runtime`.
- Added `StdioJobWorker`, a generic worker process client that exchanges
  `JobRequest<T>` / `JobResponse<U>` frames over standard streams.
- Added generic routing validation for job id, affinity key, and per-connection
  sequence numbers.
- Added Rust integration tests that start a TCP listener, call a Python Mini
  Redis worker as one concrete consumer, and validate Redis replies over a real
  socket.
