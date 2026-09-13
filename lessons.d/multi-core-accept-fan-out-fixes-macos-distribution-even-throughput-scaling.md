# multi-core — accept fan-out fixes macOS distribution; even ≠ throughput scaling

**Date:** 2026-06-14

Closing the macOS `SO_REUSEPORT` gap (multi-core PR5): on macOS/BSD a
`ShardedTcpRuntime` now uses an explicit accept fan-out — one acceptor owns the
client-facing listener and round-robins each accepted socket to a worker reactor
via `adopt_stream` (transport-platform) + `StreamMailbox::adopt_connection`
(stream-reactor). The workers bind throwaway loopback listeners and only *serve*
adopted connections. Linux keeps the kernel `SO_REUSEPORT` balancing.

Two durable lessons:

1. **Even distribution is necessary but NOT sufficient for throughput scaling.**
   The fan-out fixed the shard balance (`[0% … 100%]` → `[13% × 8]`) and made
   `conns/s` scale (connection setup parallelizes). But steady-state `req/s` for a
   trivial **echo** stayed flat — echo on loopback is latency-bound, not
   CPU-bound, so spreading connections across cores adds no throughput when the
   per-request work is near-zero. Don't read "added cores, req/s didn't move" as
   "the fan-out didn't work" — check the shard-balance column (it did) and
   remember scaling only shows up once each connection's work can saturate a core
   (real parsing, TLS, compute). A benchmark must measure a CPU-bound load to
   demonstrate throughput scaling; a benchmark must measure *distribution* to
   demonstrate the fan-out.

2. **Cross-thread fd handoff is the crux and must be TSan-clean.** The acceptor
   creates an `OwnedFd` on its thread and hands it to a worker reactor through the
   mailbox; the worker adopts it into its own kqueue. `OwnedFd` is `Send`, the fd
   is linear (consumed exactly once on every path), and the whole chain
   (acceptor → mailbox → adopt_stream) passes under ThreadSanitizer. Run the
   distribution test under `-Zsanitizer=thread -Zbuild-std` whenever this path
   changes.
