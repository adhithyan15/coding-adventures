# Mini Redis C10K Hold Results

This note records the first local C10K hold experiment for Mini Redis over the
repository TCP runtime. The goal was not to publish a universal throughput
number. The goal was to answer a narrower architecture question:

Can the current single-reactor, kqueue-backed Mini Redis server hold 10,000
concurrent TCP connections and still respond after the hold?

## Benchmark Shape

The benchmark used `code/benchmarks/mini-redis/c10k-hold.toml` with the new
`tcp-resp` `hold` mode.

For each workload, the load generator:

1. opens all requested TCP client sockets with bounded dial concurrency
2. keeps every successful socket open at the same time
3. waits for the configured hold duration
4. sends a RESP `PING` over every surviving socket
5. validates every response as `+PONG\r\n`
6. closes sockets only after validation

This intentionally avoids the short-lived connection churn that can exhaust
client-side ephemeral ports and obscure server behavior.

## Environment Class

These are exploratory local results from a development macOS machine using the
macOS/BSD kqueue backend. The exact machine identity and local paths are
intentionally omitted because benchmark history should describe reproducible
engineering conditions, not private workstation state.

Relevant characteristics:

- OS family: macOS / Darwin
- Architecture: arm64
- Logical CPUs: 12
- Open-file limit reported by `benchmark-tool doctor`: 1,048,575
- Server command shape:
  `mini-redis --host 127.0.0.1 --port {port} --max-connections 10000`
- Benchmark tool version: 0.5.0

## Results

| Workload | Connections | Dial Concurrency | Hold | Connected Before Hold | Successful PINGs | Failed Operations | Trial OK |
|---|---:|---:|---:|---:|---:|---:|---|
| `redis-hold-1k` | 1,000 | 250 | 5s | 1,000 | 1,000 | 0 | yes |
| `redis-hold-5k` | 5,000 | 500 | 10s | 5,000 | 5,000 | 0 | yes |
| `redis-hold-10k` | 10,000 | 500 | 30s | 10,000 | 10,000 | 0 | yes |

The 10K hold trial completed successfully:

| Metric | Value |
|---|---:|
| Connections requested | 10,000 |
| Connections open before hold | 10,000 |
| Hold duration | 30,000 ms |
| Successful post-hold PING responses | 10,000 |
| Failed operations | 0 |
| Trial elapsed time | 30,549.365 ms |
| Median connect time | 2.0905 ms |
| Median post-hold first-byte time | 8.8605 ms |
| Median total per-socket time | 30,469.2925 ms |

Additional 10K sample percentiles:

| Metric | p50 | p90 | p99 | max |
|---|---:|---:|---:|---:|
| `connect_ms` | 2.091 | 7.350 | 220.769 | 349.564 |
| `first_byte_ms` | 8.861 | 12.362 | 16.157 | 17.899 |

## Interpretation

The first C10K experiment passed. On this local macOS/kqueue run, the current
single-reactor Mini Redis path can hold 10,000 mostly idle TCP connections and
respond correctly after the hold.

The main issue uncovered before this benchmark was not a fundamental kqueue or
single-reactor limit. It was an application/runtime configuration gap:

- `stream-reactor` defaults to a conservative 1,024 connection cap.
- Mini Redis did not previously expose a way to raise that cap for capacity
  experiments.
- The earlier benchmark shape repeatedly opened and closed many short-lived
  sockets, which can stress client-side ephemeral ports instead of server
  capacity.

This PR addresses those gaps by adding:

- `mini-redis --max-connections`
- `tcp-resp` `hold` mode
- a dedicated C10K hold manifest

## What This Does Not Prove Yet

This is a connection-capacity and liveness proof, not an active throughput
ceiling.

It does not prove:

- that one reactor can handle 10,000 busy clients at high request rates
- that command parsing or data-store execution is optimized
- that write-heavy or slow-client workloads avoid backpressure problems
- that the same numbers hold on GitHub-hosted runners or other operating
  systems
- that the current architecture has squeezed out every inch of performance

## Next Benchmarking Questions

The next useful passes should be:

1. C10K low-rate active traffic: 10,000 held sockets sending periodic `PING`.
2. Active throughput sweep: fixed connection counts with increasing request
   rates until one reactor core saturates.
3. Pipelined throughput sweep: fewer connections, many in-flight RESP requests.
4. Slow-client/backpressure tests: prove one slow reader cannot poison the
   reactor.
5. Server-side telemetry: record RSS, fd count, per-core CPU, context switches,
   and syscall counts alongside benchmark results.

If idle C10K passes but active C10K saturates one core, the likely next
architecture investment is a multi-reactor design with connection affinity, not
a generic per-request thread pool.
