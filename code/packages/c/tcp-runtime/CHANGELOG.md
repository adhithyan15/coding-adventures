# Changelog

All notable changes to the `tcp-runtime` (C) package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to semantic versioning.

## [Unreleased]

### Added

- **Outbound mailbox** (CCPP02 port campaign follow-up). A thread-safe way for a
  worker thread to reply to a connection it cannot touch directly. `tcp_runtime_mailbox`
  returns the runtime's `tcp_mailbox`; `tcp_mailbox_send`, `tcp_mailbox_send_and_close`,
  and `tcp_mailbox_close` queue commands (send bytes / send-then-close / close) for
  a connection id, executed on the reactor thread's next `tcp_runtime_poll`.
  - The mailbox is a mutex-guarded FIFO of commands (the mutex is os-platform's
    `thread` primitive — the runtime's only concurrency dependency). Enqueue copies
    the payload; the queue is the sole shared state, so the connection table stays
    private to the reactor thread. `poll` detaches the whole queue under the lock,
    then writes each command **without holding the lock** (a producer never blocks
    on I/O). A command for an unknown/closed connection id is dropped;
    `tcp_runtime_destroy` drains and frees any still-queued commands.
  - No cross-thread wakeup yet (a self-pipe/eventfd is a follow-up): delivery is on
    the next poll — within one poll timeout under `tcp_runtime_serve` (100 ms).
  - **Cross-package build change.** `tcp_runtime.c` now references `osp_mutex_*`, so
    every package that compiles it also compiles the os-platform thread backend and
    links the OS thread library: the `run.sh`/`run.ps1` of `tcp-runtime`,
    `resp-server`, and `http-server` add `os-platform/src/thread_posix.c` (`-pthread`)
    on POSIX and `thread_windows.c` (CRT-only) on Windows.
  - Test extended (101 checks, ASan+UBSan, 0 leaks — up from 57): send delivers
    bytes with no client write, send-and-close writes then closes, close removes a
    connection, a command for an unknown id is dropped, and destroy frees a command
    left queued at teardown.

- **Connection cap** (CCPP02 port campaign follow-up). `tcp_runtime_set_max_connections`
  bounds concurrent connections (0 = unlimited, the default); at the cap a newly
  accepted connection is closed immediately (the client is refused) rather than
  tracked, so `accept` still dequeues it and the listener stops re-reporting
  readable. `tcp_runtime_connection_count` reports the current live count. Test
  extended: with a cap of 1, a second client is accepted-then-closed while the
  first is served, count stays at 1 (57 checks, ASan+UBSan, 0 leaks). Note: the
  outbound mailbox and read-pause/resume backpressure remain follow-ups (the
  mailbox needs the os-platform thread backend linked into all consumers;
  backpressure needs the per-connection-state follow-up).

- **Initial package — reactor-driven TCP server** (CCPP02 port campaign, PR 2;
  the first consumer that drives `net` + `reactor` together end-to-end). The C
  port of the Rust `tcp-runtime` crate's phase-one core: one thread on a reactor
  serves many connections instead of one blocking thread per connection.
  - `tcp_runtime_bind` (listen on host:port + register the reactor),
    `tcp_runtime_local_port`, `tcp_runtime_poll` (one reactor step: accept new
    connections and service ready ones — read → handler → write → optional
    close), `tcp_runtime_serve` (loop poll until stopped), `tcp_runtime_stop`,
    `tcp_runtime_destroy`. Reuses `osp_status`.
  - The handler mirrors the Rust `TcpHandlerResult`: `tcp_action { size_t
    write_len; int close; }` — fill a reply buffer, return how many bytes to send
    and whether to close afterwards.
  - OS-agnostic: one source file with no `#ifdef`; all per-OS code stays in `net`
    and `reactor`. Each accepted connection is a heap node used verbatim as its
    reactor token (a stable allocation, since the connection-pointer array
    reallocs), so a wait result maps to its connection in O(1); the listener uses
    the runtime pointer as a distinct sentinel token.
  - Test (`tests/tcp_runtime_test.c`): stands up a real server and drives it with
    real loopback clients in one thread (stepping via `tcp_runtime_poll`),
    proving multiplexing (two independent connections accepted + echoed on one
    reactor) and echo-and-close. Verified under ASan+UBSan with 0 leaks.
  - Scope: phase-one core. Deferred (mirroring the Rust crate's phased plan):
    per-connection state, an outbound mailbox for worker threads, read
    pause/resume backpressure (`defer_read`), socket-option policy, connection
    caps, and multi-core reactor sharding; replies over the 8 KiB per-read buffer
    are truncated.
