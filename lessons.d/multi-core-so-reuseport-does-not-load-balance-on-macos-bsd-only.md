# multi-core — SO_REUSEPORT does NOT load-balance on macOS/BSD (only Linux)

**Date:** 2026-06-14

The `sharded-echo-bench` (multi-core PR4) measured the `ShardedTcpRuntime` at
1/2/4/8 reactor shards and the per-shard accept-balance column revealed a hard
truth: on **macOS** every connection lands on a **single** shard (`[0% 0% 0% 100%]`),
so throughput is flat (8-shard ≈ 0.97× single-shard). Plain `SO_REUSEPORT` only
load-balances on **Linux** (kernel hash across the reuseport group); macOS/*BSD
permit the multi-bind but deliver all connections to one socket (in practice the
last bound), and FreeBSD needs the separate `SO_REUSEPORT_LB` option.

Consequences:
- The N-reactor + SO_REUSEPORT design scales on **Linux** (the deployment target
  and where CI runs) but gives **no** accept distribution on a macOS dev box.
  Don't read a flat local (macOS) scaling curve as "the runtime doesn't scale" —
  check the shard-balance column first; it's a kernel policy, not a runtime bug.
- True multi-core accept distribution on macOS/BSD needs an explicit fan-out (a
  single accept loop that round-robins accepted fds to per-core reactors, or
  `SO_REUSEPORT_LB`), not reliance on the kernel balancing plain `SO_REUSEPORT`.
- Lesson for benchmarks: always surface the *distribution*, not just the average.
  A single throughput number would have hidden this; the per-shard column made it
  obvious in one glance.
