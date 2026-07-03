# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-18

### Added

- `StreamReactor` generic over `transport-platform`
- neutral `StreamHandlerResult` for bytes plus close intent
- connection caps and queued-write budget caps
- macOS/BSD `bind_kqueue` convenience constructor
- macOS/BSD end-to-end echo and budget/cap/shutdown tests

## [0.1.1] - 2026-04-18

### Added

- `bind_with_state` and `bind_kqueue_with_state` for connection-local
  application state
- close callbacks that receive the final connection state exactly once
- tests covering state persistence across reads and close callback teardown

### Fixed

- stabilized the stateful read/write test to tolerate delayed client-side
  readability on CI

## [0.1.2] - 2026-04-21

### Added

- Added a thread-safe outbound mailbox so off-reactor worker threads can queue
  writes or close requests by `ConnectionId`.
- Added tests for delayed mailbox writes, write-and-close delivery, and stale
  mailbox commands for already closed streams.

## [0.1.3] - 2026-04-22

### Added

- Added mailbox read pause/resume commands for backpressure-aware adapters.
- Added deferred-read replay so a handler can ask the reactor to retry an
  already-read byte chunk after reads are resumed.
- Added a regression test proving deferred bytes are not lost across pause and
  resume.

## [0.1.4] - 2026-06-14

### Changed

- **Shard-aware `ConnectionId` allocation.** Each reactor now stamps its shard
  index into the low `shard_bits` of every `ConnectionId` it allocates
  (`id = (sequence << shard_bits) | shard_index`), so a multi-reactor mailbox can
  route a write to the one reactor that owns a connection with pure arithmetic and
  no shared registry. A lone reactor uses `shard_bits == 0`, leaving ids
  byte-identical to the previous sequence-counter scheme.
- Replaced the `StreamReactorOptions::connection_id_seed` shared `AtomicU64` with
  `shard_index` / `shard_bits` fields. Uniqueness across shards no longer needs a
  cross-shard atomic on the accept hot path — the shard bits plus each reactor's
  private sequence are unique by construction.

### Added

- Regression test `connection_ids_encode_the_shard_index_in_their_low_bits`.

## [0.1.5] - 2026-06-14

### Added

- **Cross-thread reactor wakeup.** Each reactor registers a wakeup at construction
  and hands its `StreamMailbox` a `WakeHandle`; `StreamMailbox::push` fires it
  after enqueuing, so an off-reactor write interrupts `poll` and flushes in
  milliseconds instead of waiting out the poll timeout (default 10 ms). Best-effort
  and non-fatal: on platforms whose wakeup can't be shared across threads, the
  mailbox simply has no handle and the reactor drains on its next poll as before.
- Tests: `off_reactor_mailbox_write_wakes_the_reactor_immediately` (a 30 s poll
  timeout proves the wake fires — a dropped wake fails the test, not just slows it)
  and `concurrent_off_reactor_wakes_do_not_corrupt_the_reactor` (8 threads hammer
  the wake while clients echo). Both pass under ThreadSanitizer.

## [0.1.6] - 2026-06-14

### Added

- **`StreamMailbox::adopt_connection(fd, peer_addr)`** — the receiving half of an
  accept fan-out. An acceptor thread accepts on a single listener and round-robins
  each accepted socket to a worker reactor's mailbox; the reactor adopts the `fd`
  via `TransportPlatform::adopt_stream` and then serves it like any connection it
  accepted itself. The enqueue wakes the reactor (PR2 wake handle), so adoption is
  prompt. Admission control (the connection cap) is enforced in the reactor: a
  full reactor drops the `fd` (closing the socket) rather than admitting it; a
  failed adopt drops just that one connection instead of tearing down the reactor.
- New `AdoptConnection { fd, peer_addr }` mailbox command, routed in
  `drain_mailbox` (it mints a fresh `ConnectionId`, so it bypasses the
  `ConnectionId`-keyed dispatch path).

### Changed

- Extracted `register_connection(stream, peer_addr)` — the shared tail that
  configures a stream, sets readable interest, mints the shard-encoded
  `ConnectionId`, and builds per-connection state — so `accept_ready` and the new
  adopt path produce identical connection state. No behavior change for accept.

### Tested

- `adopts_an_externally_accepted_connection_via_the_mailbox`: a separate listener
  accepts a real loopback connection, the socket is handed to the reactor via
  `adopt_connection`, and the reactor echoes it. Passes under ThreadSanitizer
  (the `OwnedFd` crosses threads through the mailbox).

## [0.1.7] - 2026-06-14

### Added

- `StreamReactorOptions::accept_connections` (default `true`). When `false`, the
  reactor still binds a listener (the constructor requires one) but never enables
  accept interest on it, so it serves only adopted connections — used by the
  macOS/BSD fan-out worker reactors so a direct connect to a worker's throwaway
  port is left unaccepted (closing it as an ingress) rather than served.
