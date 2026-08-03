# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

- Keep the BSD-only accept fan-out state clean under Rust 1.97 when compiling
  the Windows transport dependency closure.

## [0.1.0] - 2026-04-18

### Added

- `TcpRuntime` as the first TCP-specific runtime facade above `stream-reactor`
- `TcpRuntimeOptions` for listener policy, stream policy, and runtime limits
- `TcpConnectionInfo` with both peer and local listener addresses
- `TcpHandlerResult` for queued writes plus close-after-flush intent
- host-OS convenience constructors for `kqueue`, `epoll`, and Windows transport
  providers
- macOS / BSD end-to-end tests for echo behavior, local-address metadata,
  connection caps, queued-write overflow, and stop-handle shutdown

## [0.1.1] - 2026-04-18

### Added

- `bind_with_state` plus stateful OS convenience constructors for protocol
  session state
- close callbacks that observe final TCP connection state during teardown
- tests proving stateful handlers preserve per-connection state across multiple
  reads

## [0.1.2] - 2026-04-21

### Added

- Added a TCP outbound mailbox wrapper so worker threads can send delayed bytes
  or close requests by `ConnectionId`.
- Added tests proving TCP reads can return immediately while another thread
  posts the eventual response.

## [0.1.3] - 2026-04-22

### Added

- Added TCP mailbox read pause/resume helpers for worker-backed adapters.
- Added `TcpHandlerResult::defer_read()` so adapters can pause a connection
  and replay the already-read bytes later instead of dropping data or closing
  on backpressure.

## [0.1.4] - 2026-06-14

### Changed

- **`TcpMailbox` now routes instead of broadcasting.** `send` / `send_and_close` /
  `close` / `pause_reads` / `resume_reads` decode the owning shard from the
  `ConnectionId` (low `shard_bits`) and enqueue into that one reactor's mailbox,
  instead of cloning the bytes into every reactor's queue and having non-owners
  drop them. `resume_all_reads` (which carries no id) still fans out to all
  reactors. Single-reactor runtimes have `shard_bits == 0`, so routing is a no-op
  index of `0`.
- Sharded builders assign each worker a `shard_index` and a shared
  `shard_bits = ceil(log2(worker_count))`, and no longer create the shared
  `AtomicU64` connection-id seed (uniqueness now comes from the `ConnectionId`
  shard encoding).

### Added

- `shard_bits_for(worker_count)` helper and unit test `shard_bits_for_is_ceil_log2`.
- End-to-end test `sharded_mailbox_routes_replies_through_the_owning_shard`:
  replies routed only through the mailbox reach the correct connection across a
  4-shard runtime (a wrong shard mask would hang the client read).

## [0.1.5] - 2026-06-14

### Added

- **Accept fan-out on macOS/BSD.** `ShardedTcpRuntime` with `worker_count > 1` now
  distributes connections with an explicit acceptor instead of relying on
  `SO_REUSEPORT` (which doesn't load-balance on Darwin/BSD — it delivers every
  connection to one socket). A single `FanoutAcceptor` owns the client-facing
  listener and round-robins each accepted socket to a worker reactor via
  `StreamMailbox::adopt_connection`; the worker reactors bind throwaway loopback
  listeners and only *serve* the connections handed to them. Linux keeps the
  simpler `SO_REUSEPORT` path (the kernel balances there). `ShardedStopHandle` /
  `stop` now also signal the acceptor's stop flag; `serve` runs and joins it.
- Test `sharded_runtime_distributes_connections_across_shards`: with 3 shards and
  12 clients, connections land on more than one shard — the regression guard that
  the old all-on-one-shard macOS behavior would fail. Stress-looped (8×, stable);
  the acceptor's cross-thread `OwnedFd` handoff passes under ThreadSanitizer.

### Result

- `sharded-echo-bench` on macOS now reports an even shard balance
  (`[13% 13% …]`) where it previously showed `[0% … 100%]`, and `conns/s` scales
  with shards (connection setup parallelizes). Steady-state `req/s` for a trivial
  echo stays flat — that workload is latency-bound on loopback, not CPU-bound, so
  even distribution doesn't add throughput until the per-connection work is heavy
  enough to saturate a core.
