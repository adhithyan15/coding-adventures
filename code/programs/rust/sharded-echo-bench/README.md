# sharded-echo-bench

A scaling benchmark for the multi-core [`ShardedTcpRuntime`](../../../packages/rust/tcp-runtime):
does adding reactor shards actually add throughput?

It stands up a trivial echo server at `worker_count = 1, 2, 4, 8`, drives each with
a pool of real loopback TCP clients doing request/response echoes for a fixed
duration, and prints:

```
 workers |   conns/s |     req/s |   MiB/s |  p50 µs |  p99 µs | shard balance
---------+-----------+-----------+---------+---------+---------+---------------
       1 |     41127 |    128446 |     7.8 |   466.3 |   701.3 | [100%]
       2 |     66079 |    128221 |     7.8 |   473.6 |   663.9 | [0% 100%]
       4 |     66413 |    126182 |     7.7 |   483.5 |   667.4 | [0% 0% 0% 100%]
       8 |     55644 |    125082 |     7.6 |   503.1 |   667.2 | [0% 0% 0% 0% 0% 0% 0% 100%]
```

(macOS run, 14 cores — see the finding below.)

## Run it

The full sweep is **opt-in** (it saturates cores for a few seconds per shard
count), so it is *not* what CI runs — CI runs only a fast `#[test]` smoke:

```sh
cargo run -p sharded-echo-bench --release
# tunables:
BENCH_CLIENTS=128 BENCH_PAYLOAD=64 BENCH_SECONDS=3 cargo run -p sharded-echo-bench --release
```

## The `shard balance` column is the headline

`ShardedTcpRuntime` runs *N* reactors and asks the kernel to spread accepted
connections across them via `SO_REUSEPORT`. The benchmark prints how many
connections each shard actually accepted, so you can see whether that spreading
is happening — instead of blaming a flat throughput curve on the runtime.

### Finding: `SO_REUSEPORT` load-balances on Linux, **not** on macOS/BSD

The sample above is from macOS, and the balance column is the punchline:
`[0% 0% 0% 100%]` — **every** connection went to a single shard, so the other
reactor threads sit idle and throughput is flat (`8-shard ≈ 0.97× single-shard`).

This is a kernel-policy difference, not a bug in the runtime:

| OS | plain `SO_REUSEPORT` behavior |
|----|-------------------------------|
| **Linux** | distributes new connections across the reuseport group by a kernel hash — balanced, so shards scale |
| **macOS / *BSD** | permits the multi-bind but delivers all connections to **one** socket (in practice the last bound) — no distribution |
| **FreeBSD** | balanced *only* with `SO_REUSEPORT_LB` (a separate option) |

So the multi-core runtime scales on **Linux** (the deployment target, and where
CI runs); on macOS it binds N reactors but the kernel hands them all to one. The
benchmark makes that explicit rather than hiding it behind an averaged number.

**Future work** (not this crate): for real multi-core accept distribution on
macOS/BSD, the runtime would need an explicit fan-out — e.g. a single accept loop
that round-robins accepted fds to per-core reactors, or `SO_REUSEPORT_LB` on
FreeBSD — instead of relying on the kernel to balance plain `SO_REUSEPORT`.

## Reading the rest honestly

- **It measures the runtime, not a NIC.** Everything is loopback, so there is no
  wire — this isolates reactor/scheduling cost. Real-NIC numbers differ.
- **Clients share the cores with the server.** The load generator runs in-process,
  so on a `C`-core box the server can't truly use more than `C` cores once the
  clients are also busy. Read the *curve*, not the absolute ceiling.
- **`p50`/`p99`** are nearest-rank percentiles over every recorded round-trip.

## How it fits

```
sharded-echo-bench (this crate — load generator + echo server + report)
        ↓ exercises
ShardedTcpRuntime  (tcp-runtime: N reactors, SO_REUSEPORT, shard-routed mailbox)
        ↓
transport-platform → kqueue / epoll / IOCP
```
