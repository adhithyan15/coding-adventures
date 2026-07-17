# Changelog

All notable changes to the `tcp-runtime` (C) package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to semantic versioning.

## [Unreleased]

### Added

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
