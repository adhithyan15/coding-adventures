# Changelog

All notable changes to this package will be documented in this file.

## [0.2.0] - 2026-06-14

### Added

- **CPU-bound workload mode** (`BenchParams::work_per_request`, env var
  `BENCH_WORK`, default `0`). The echo handler optionally does `N` rounds of a
  cheap hash over the payload before replying, turning each request from a
  latency-bound loopback round-trip into real CPU work. This is the regime where
  adding reactor shards actually adds throughput.
- **The throughput-scaling proof.** With `BENCH_WORK=20000` on a 14-core macOS box,
  `req/s` scales near-linearly with shards — `800 → 1588 → 2962 → 5830` across
  `1 → 2 → 4 → 8` shards (≈7.3× at 8×), with `p50` latency falling in lockstep
  (39.5 ms → 5.4 ms). The default `BENCH_WORK=0` run stays flat; the two side by
  side close the loop the 0.1.0 finding left open ("even distribution is necessary
  but not sufficient — you need CPU-bound work to see req/s scale").
- Test `cpu_bound_mode_still_echoes_correctly` — proves the CPU-work path doesn't
  corrupt the response (bytes still round-trip) and the run still reports.

### Changed

- `run_sweep` gains a `work_per_request` parameter; the CLI prints the
  `work/req` setting and a one-line regime description (latency-bound vs CPU-bound).

## [0.1.0] - 2026-06-14

### Added

- `sharded-echo-bench` — a scaling benchmark for the multi-core
  `ShardedTcpRuntime`. Stands up a loopback echo server at
  `worker_count = 1, 2, 4, 8`, drives each with a pool of real TCP clients doing
  request/response echoes for a fixed duration, and reports a table of
  connections/sec, requests/sec, MiB/s, p50/p99 round-trip latency, and — the
  headline — the **per-shard accept balance** so `SO_REUSEPORT` distribution is
  visible rather than averaged away.
- `run_benchmark` / `run_sweep` / `format_table` library API (so the harness is
  unit-testable) plus a CLI (`cargo run`) whose full sweep is opt-in; CI runs only
  the fast `#[test]` smoke (2 shards, a few clients, 300 ms).
- Tests: `smoke_two_shards_echo_and_report`, `single_shard_runs`,
  `percentiles_handle_empty_and_ordering`, `shard_bits_matches_ceil_log2`.

### Findings

- **`SO_REUSEPORT` load-balances on Linux but not on macOS/BSD.** The shard-balance
  column showed `[0% … 100%]` on macOS — every connection lands on one shard, so
  throughput is flat there (`8-shard ≈ 0.97× single-shard`). On Linux the kernel
  distributes the reuseport group and shards scale. This is a kernel-policy
  difference, not a runtime bug; the README documents it and flags the macOS
  fix (explicit accept fan-out / `SO_REUSEPORT_LB`) as future work.
