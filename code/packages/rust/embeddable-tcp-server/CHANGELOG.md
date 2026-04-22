# Changelog

All notable changes to this package will be documented in this file.

## [0.1.1] - 2026-04-22

### Added

- Added `worker_job_timeout` to mailbox-mode server options so embedders can
  configure generic job-runtime timeouts for stuck language workers.
- Added `worker_queue_depth` to mailbox-mode server options so embedders can
  tune the bounded worker queue used for backpressure.
- Added queue-full handling that defers and pauses the current TCP read, then
  resumes paused reads when worker responses release queue capacity.
- Added an integration test proving a second connection survives worker queue
  pressure and receives its replayed response.

## [0.1.0] - 2026-04-20

### Added

- Added `EmbeddableTcpServer`, a language-neutral TCP bridge built on
  `tcp-runtime`.
- Added `StdioJobWorker`, a generic worker process client that exchanges
  `JobRequest<T>` / `JobResponse<U>` frames over standard streams.
- Added generic response id validation for stdio worker replies.
- Added Rust integration tests that start a TCP listener, call a Python Mini
  Redis worker as one concrete consumer, and validate Redis replies over a real
  socket.
- Updated the Mini Redis integration so Rust sends only opaque TCP byte jobs
  and writes opaque byte frames. The Python worker owns RESP framing,
  per-stream selected database state, and RESP response assembly.
- Added a mailbox-style asynchronous worker path where TCP callbacks send
  request jobs and return immediately while a response task posts worker output
  back to the TCP runtime.
- Routed mailbox-mode worker execution through `generic-job-runtime` so the
  embeddable TCP server can use a configurable stdio process pool instead of
  owning one ad hoc child process.
