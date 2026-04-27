# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Added

- Added `new_inprocess_mailbox` in `EmbeddableTcpServer` to run job execution in
  `generic-job-runtime`'s `RustThreadPool` while keeping the TCP callback and
  mailbox return path unchanged.
- Added `RustThreadPoolJobSubmitter` to decouple job IDs and in-process route
  tracking from the stdio worker path.
- Added a `build_runtime_mailbox` path that can consume either stdio-backed or
  thread-pool-backed mailbox submitters.
- Added integration coverage for the in-process thread-pool mailbox path.

## [0.1.1] - 2026-04-22

### Added

- Added `worker_job_timeout` to mailbox-mode server options so embedders can
  configure generic job-runtime timeouts for stuck language workers.
- Added `worker_restart_policy` to mailbox-mode server options so embedders can
  opt into generic job-runtime process restart behavior.
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
