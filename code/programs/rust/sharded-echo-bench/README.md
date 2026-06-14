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
# make each request CPU-bound so req/s scales with shards (see the throughput proof below):
BENCH_WORK=20000 cargo run -p sharded-echo-bench --release
```

`BENCH_WORK` (default `0`) is rounds of synthetic CPU work per request: `0` is a
pure echo (latency-bound), non-zero makes the load CPU-bound.

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

The benchmark makes that explicit rather than hiding it behind an averaged number.

### Resolved: macOS/BSD now distribute via an accept fan-out

`tcp-runtime` since 0.1.5 closes this gap: on macOS/BSD a `ShardedTcpRuntime` with
`worker_count > 1` uses an explicit **accept fan-out** — one acceptor owns the
client-facing listener and round-robins each accepted socket to a worker reactor
(via `adopt_stream` / `StreamMailbox::adopt_connection`) — instead of relying on
the kernel to balance plain `SO_REUSEPORT`. With the fan-out the macOS shard
balance is now even:

```
 workers |   conns/s |     req/s |   MiB/s |  p50 µs |  p99 µs | shard balance
       1 |     40569 |    111142 |     6.8 |   604.0 |   824.6 | [100%]
       2 |     64655 |    108901 |     6.6 |   587.3 |   753.0 | [50% 50%]
       4 |     70556 |    104371 |     6.4 |   605.2 |   853.5 | [25% 25% 25% 25%]
       8 |     76954 |     99503 |     6.1 |   619.3 |  1131.1 | [13% 13% 13% 13% 13% 13% 13% 13%]
```

`conns/s` now scales with shards (connection *setup* parallelizes, 40k → 77k).
But note the steady-state `req/s` for this trivial echo is still flat — that's
expected and honest: echo on loopback is **latency-bound, not CPU-bound**, so even
distribution doesn't add throughput until the per-connection work is heavy enough
to saturate a core (real parsing, TLS, compute…). Even distribution is necessary
for multi-core scaling; it is not sufficient on its own for a near-zero workload.
Linux uses the kernel `SO_REUSEPORT` balancing and reaches the same even
distribution by a different route.

### The throughput proof: `BENCH_WORK` makes requests CPU-bound

The claim above — "even distribution adds throughput *once the work saturates a
core*" — is testable, so the benchmark tests it. `BENCH_WORK=N` makes the echo
handler do `N` rounds of a cheap hash over the payload **before** replying, turning
each request from a near-free loopback round-trip into real CPU work. Now the
shards have something to parallelize, and `req/s` scales with the worker count:

```
$ BENCH_CLIENTS=32 BENCH_WORK=20000 cargo run -p sharded-echo-bench --release   # macOS, 14 cores

 workers |   conns/s |     req/s |   MiB/s |  p50 µs |  p99 µs | shard balance
---------+-----------+-----------+---------+---------+---------+---------------
       1 |     36495 |       800 |     0.0 | 39543.7 | 47535.4 | [100%]
       2 |     32686 |      1588 |     0.1 | 20229.1 | 21574.2 | [50% 50%]
       4 |     31739 |      2962 |     0.2 | 10724.2 | 14313.7 | [25% 25% 25% 25%]
       8 |     34985 |      5830 |     0.4 |  5423.8 |  6736.9 | [13% 13% 13% 13% 13% 13% 13% 13%]

8-shard throughput is 7.29x the single-shard baseline.
```

That is the multi-core payoff, end to end: **800 → 1588 → 2962 → 5830 req/s** is
near-linear scaling across `1 → 2 → 4 → 8` shards (≈7.3× at 8×), and `p50` latency
falls in lockstep (39.5 ms → 5.4 ms) because eight reactors are draining the same
offered load in parallel. Run the same sweep with `BENCH_WORK=0` (the default) and
`req/s` stays flat — the two runs side by side are the whole thesis: a multi-core
runtime turns parallel-izable work into throughput, and trivial work has none to
turn.

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
