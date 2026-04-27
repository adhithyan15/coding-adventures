# Mini Redis TCP Runtime Results

This file records the first local C10K experiments for Mini Redis running on the
repository TCP runtime.

## Summary

Mini Redis can hold 10,000 concurrent TCP connections on the macOS/kqueue
runtime and continue to answer requests correctly.

The event loop is not the first bottleneck we found. The first clear bottleneck
appears when 10,000 held connections perform mixed Redis-style operations. That
workload remains correct, but tail latency rises to seconds while the server
process saturates roughly one CPU core.

## Environment

These are local exploratory results, not portable throughput claims.

| Field | Value |
|---|---|
| OS family | macOS / Darwin |
| Architecture | arm64 |
| Event backend | kqueue |
| Logical CPUs | 12 |
| Open-file limit | 1,048,575 |
| Server shape | `mini-redis --host 127.0.0.1 --port {port} --max-connections 10000` |
| Benchmark tool | `0.5.0` |

## Workloads

| Workload | Purpose |
|---|---|
| Idle hold | Open many sockets, hold them, then prove each socket still responds to `PING`. |
| Low-rate active | Keep sockets open and send periodic `PING` traffic across every connection. |
| Mixed operations | Keep sockets open and execute randomized supported commands on every connection. |

Mixed operations used per-connection keys and validated every RESP response.
The operation mix was `PING`, `SET`, `GET`, `EXISTS`, `INCRBY`, `HSET`, `HGET`,
and `HEXISTS`.

## Results

### Idle Hold

| Run | Connections | Hold | Successful PINGs | Failures | p99 first byte |
|---|---:|---:|---:|---:|---:|
| `redis-hold-1k` | 1,000 | 5s | 1,000 | 0 | not recorded |
| `redis-hold-5k` | 5,000 | 10s | 5,000 | 0 | not recorded |
| `redis-hold-10k` | 10,000 | 30s | 10,000 | 0 | 16.157 ms |

The 10K idle hold passed: all 10,000 sockets remained open through the hold and
all 10,000 post-hold `PING` requests returned `+PONG`.

### Low-Rate Active PING

| Run | Connections | Total PINGs | Failures | p50 | p90 | p99 | max |
|---|---:|---:|---:|---:|---:|---:|---:|
| `active-1k` | 1,000 | 5,000 | 0 | 9.814 ms | 15.939 ms | 18.841 ms | 22.161 ms |
| `active-5k` | 5,000 | 25,000 | 0 | 9.387 ms | 20.350 ms | 27.322 ms | 28.359 ms |
| `active-10k` | 10,000 | 50,000 | 0 | 10.113 ms | 18.516 ms | 27.312 ms | 32.207 ms |

The 10K active PING run passed: 10,000 held sockets completed 50,000 validated
`PING` requests with zero failures.

### Mixed Operations

| Run | Connections | Total Ops | Failures | p50 | p90 | p99 | max |
|---|---:|---:|---:|---:|---:|---:|---:|
| `mixed-1k` | 1,000 | 5,000 | 0 | 169.317 ms | 293.159 ms | 294.362 ms | 297.912 ms |
| `mixed-5k` | 5,000 | 20,000 | 0 | 1,393.117 ms | 2,474.768 ms | 2,594.699 ms | 2,597.052 ms |
| `mixed-10k` | 10,000 | 20,000 | 0 | 1,474.075 ms | 2,690.686 ms | 2,899.111 ms | 2,906.966 ms |

The mixed workload also passed correctness checks. The server held 10,000
connections and completed 20,000 validated mixed operations with zero failures.

The cost is latency. During the 10K mixed run, the Mini Redis process was
sampled at roughly one saturated CPU core:

| Metric | Observed |
|---|---:|
| CPU median | 97.7% |
| CPU max | 104.0% |
| RSS median | 22.0 MiB |
| RSS max | 26.0 MiB |

## Conclusion

The current TCP runtime has cleared the first C10K bar: it can hold 10,000
connections and service low-rate traffic without correctness failures.

The next performance problem is active command execution, not idle connection
capacity. Mixed Redis-style operations remain correct at 10K, but p99 latency
reaches about 2.9 seconds while the server is effectively pinned to one core.

Before investing directly in a thread pool, the next pass should profile the
Mini Redis hot path. The likely suspects are RESP parsing/encoding, serialized
command execution, and whole-store clone/writeback behavior inside the
datastore layer. If that hot path is improved and one core still saturates, the
next architecture step should be multi-reactor networking, datastore sharding,
or worker execution with clear connection affinity.
