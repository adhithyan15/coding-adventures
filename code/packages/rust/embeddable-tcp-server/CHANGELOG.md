# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Added

- **WEB01b-1b: `ordered_responses` option** for the in-process mailbox. When set,
  the response router writes each connection's responses back in **submission
  order** (not worker-completion order) via a per-connection reorder buffer: the
  submitter records each connection's job-ids in submission order, and the router
  buffers a finished response until every earlier one on the same connection has
  been written. This is what a pipelined HTTP/1.1 keep-alive connection needs — the
  pool is unordered (`supports_ordered_responses: false`), so without it a
  connection's replies could be written out of request order. Default `false`
  preserves the exact completion-order behaviour for all other consumers
  (`MailboxHttpServer` is the only opt-in). New test
  `inprocess_mailbox_orders_responses_by_submission_when_enabled` proves it
  deterministically (workers finish `3,2,1,0`; the wire reads `0,1,2,3`).
- Added `new_inprocess_mailbox` in `EmbeddableTcpServer` to run job execution in
  `generic-job-runtime`'s `RustThreadPool` while keeping the TCP callback and
  mailbox return path unchanged.
- Added `RustThreadPoolJobSubmitter` to decouple job IDs and in-process route
  tracking from the stdio worker path.
- Added a `build_runtime_mailbox` path that can consume either stdio-backed or
  thread-pool-backed mailbox submitters.
- Added integration coverage for the in-process thread-pool mailbox path.
- Added an ignored in-process mailbox stress test for configurable concurrent
  real TCP clients without requiring a Python worker.

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
