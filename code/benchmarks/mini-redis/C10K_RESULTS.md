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

## Follow-Up: Low-Rate Active C10K

A second local exploratory run kept the same server binary alive and sent real
RESP `PING` traffic while the sockets were held open. This used a temporary
external load driver so we could learn quickly without expanding the committed
benchmark-tool surface before deciding on the final active-workload contract.

The active driver:

1. opened the requested number of sockets once
2. held the sockets open
3. repeatedly swept across every socket
4. sent one RESP `PING` per socket per sweep
5. required every response to be `+PONG\r\n`

### Active Run Shape

| Run | Connections | Dial Concurrency | Operation Concurrency | Duration | Sweep Interval | Total Sweeps |
|---|---:|---:|---:|---:|---:|---:|
| `active-1k` | 1,000 | 250 | 500 | 15s | 3s | 5 |
| `active-5k` | 5,000 | 500 | 1,000 | 20s | 4s | 5 |
| `active-10k` | 10,000 | 500 | 1,000 | 25s | 5s | 5 |

### Active Run Results

| Run | Opened | Dial Failures | Total PINGs | Failed PINGs | Result |
|---|---:|---:|---:|---:|---|
| `active-1k` | 1,000 | 0 | 5,000 | 0 | pass |
| `active-5k` | 5,000 | 0 | 25,000 | 0 | pass |
| `active-10k` | 10,000 | 0 | 50,000 | 0 | pass |

### Active Latency Summary

Aggregate PING latency across all waves:

| Run | p50 | p90 | p99 | max |
|---|---:|---:|---:|---:|
| `active-1k` | 9.814 ms | 15.939 ms | 18.841 ms | 22.161 ms |
| `active-5k` | 9.387 ms | 20.350 ms | 27.322 ms | 28.359 ms |
| `active-10k` | 10.113 ms | 18.516 ms | 27.312 ms | 32.207 ms |

Dial latency for the active runs:

| Run | p50 | p90 | p99 | max |
|---|---:|---:|---:|---:|
| `active-1k` | 4.235 ms | 55.612 ms | 56.809 ms | 85.275 ms |
| `active-5k` | 1.805 ms | 33.732 ms | 125.586 ms | 157.764 ms |
| `active-10k` | 1.912 ms | 7.207 ms | 345.044 ms | 595.906 ms |

### Active Run Interpretation

The low-rate active C10K run also passed. The server held 10,000 concurrent
connections while processing 50,000 validated RESP `PING` operations over those
connections with zero failures.

This is stronger than the idle C10K proof because the held sockets were used
throughout the benchmark. It still is not a maximum-throughput result: each
connection only sent one `PING` every five seconds in the 10K run. The next
step is to formalize this active mode in `benchmark-tool` and sweep request
rates upward until latency or one reactor core clearly saturates.

## Follow-Up: Mixed Supported Operations

A third local exploratory run kept thousands of sockets open and made every
connection execute randomized supported Mini Redis commands instead of only
`PING`.

The temporary driver used deterministic per-connection random streams and
per-connection unique keys so correctness could be checked without introducing
cross-connection key races. Each operation was sent as a RESP command and every
response was validated.

Operations in the mix:

- `PING`
- `SET`
- `GET`
- `EXISTS`
- `INCRBY`
- `HSET`
- `HGET`
- `HEXISTS`

### Mixed Run Shape

| Run | Connections | Dial Concurrency | Operation Concurrency | Duration | Sweep Interval |
|---|---:|---:|---:|---:|---:|
| `mixed-1k` | 1,000 | 250 | 500 | 15s | 3s |
| `mixed-5k` | 5,000 | 500 | 1,000 | 20s | 4s |
| `mixed-10k` | 10,000 | 500 | 1,000 | 15s | 5s |

### Mixed Run Results

| Run | Opened | Dial Failures | Waves | Total Operations | Failed Operations | Result |
|---|---:|---:|---:|---:|---:|---|
| `mixed-1k` | 1,000 | 0 | 5 | 5,000 | 0 | pass |
| `mixed-5k` | 5,000 | 0 | 4 | 20,000 | 0 | pass |
| `mixed-10k` | 10,000 | 0 | 2 | 20,000 | 0 | pass |

Aggregate mixed-operation latency:

| Run | p50 | p90 | p99 | max |
|---|---:|---:|---:|---:|
| `mixed-1k` | 169.317 ms | 293.159 ms | 294.362 ms | 297.912 ms |
| `mixed-5k` | 1,393.117 ms | 2,474.768 ms | 2,594.699 ms | 2,597.052 ms |
| `mixed-10k` | 1,474.075 ms | 2,690.686 ms | 2,899.111 ms | 2,906.966 ms |

The mixed run stayed correct, but latency increased sharply as state and
connection count grew. Per-wave latency shows the ramp clearly:

| Run | Wave | Operations | p50 | p90 | p99 | Max |
|---|---:|---:|---:|---:|---:|---:|
| `mixed-1k` | 1 | 1,000 | 39.820 ms | 65.955 ms | 66.456 ms | 66.512 ms |
| `mixed-1k` | 5 | 1,000 | 293.159 ms | 293.901 ms | 294.516 ms | 297.912 ms |
| `mixed-5k` | 1 | 5,000 | 402.435 ms | 606.605 ms | 660.432 ms | 688.515 ms |
| `mixed-5k` | 4 | 5,000 | 2,471.861 ms | 2,587.695 ms | 2,596.911 ms | 2,597.052 ms |
| `mixed-10k` | 1 | 10,000 | 757.779 ms | 1,361.560 ms | 1,428.895 ms | 1,489.817 ms |
| `mixed-10k` | 2 | 10,000 | 2,274.416 ms | 2,813.711 ms | 2,902.143 ms | 2,906.966 ms |

The 10K mixed run's Mini Redis process was sampled while the workload ran:

| Metric | Observed |
|---|---:|
| CPU min | 0.0% |
| CPU median | 97.7% |
| CPU max | 104.0% |
| RSS min | 1.9 MiB |
| RSS median | 22.0 MiB |
| RSS max | 26.0 MiB |

### Mixed Run Interpretation

The mixed workload proves the server can still hold 10,000 connections and
remain correct while those connections perform real supported Redis-style work.
It also exposes the next bottleneck much more clearly than the idle and `PING`
runs.

The server appears CPU-bound on roughly one core under the mixed workload. This
matches the current architecture:

- the TCP runtime is a single-reactor path
- Mini Redis calls into one shared `DataStoreManager`
- `DataStoreEngine::execute_parts_with_db` clones the current store, runs the
  command, then writes the new store back
- the background expiration loop also periodically clones/updates store state

That architecture is excellent for proving correctness and the eventing seam,
but it is not the final high-throughput data-store execution architecture.

The next optimization question is not "can we hold C10K?" anymore. We can. The
next question is: how do we keep active mixed workloads from saturating one core
and turning each operation wave into seconds of tail latency?

Likely next investigations:

1. Profile the mixed workload to quantify time in RESP parsing, command
   execution, store cloning, and response encoding.
2. Replace whole-store clone/writeback in the hot path with in-place mutation or
   finer-grained copy-on-write.
3. Add server-side benchmark telemetry to capture per-core CPU and command
   counts directly in result artifacts.
4. Once the datastore hot path is less clone-heavy, rerun active C10K before
   deciding whether the next architecture step is multi-reactor networking,
   datastore sharding, or worker execution.
