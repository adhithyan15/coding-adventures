# WEB01 — Async / Parallel `web-core`

> **Design spec (specs-first).** No code yet. This document scopes the one
> remaining WEB item from the Conduit completeness audit: making `web-core`
> handle requests **in parallel** instead of one-at-a-time. It lays out two
> implementation paths, recommends one, and phases the work into PRs. The
> WEB00 spec reserved this as "Phase 3: Async promotion (WEB01)".

## Purpose

Today `web-core` serves requests **synchronously on the I/O thread**: the HTTP
server calls `app.handle(request)` inline and writes the response before reading
the next request. A slow or CPU-bound handler therefore blocks the reactor — one
expensive request stalls every other connection. WEB01 makes request handling
**parallel** so the framework (and every one of the 16 language ports that runs
on it) scales across cores, with **no change to any language-layer DSL**.

## Current state (what serializes)

`WebServer::bind` ([`web-core/src/server.rs`](../packages/rust/web-core/src/server.rs))
wires the app into the HTTP server as a single inline closure:

```rust
let inner = HttpServer::bind(platform, address, options, move |request| {
    app_clone.handle(request)   // ← runs on the reactor's I/O thread, inline
})?;
```

`embeddable-http-server`'s `HttpServer` takes a
`Fn(HttpRequest) -> HttpResponse + Send + Sync` and invokes it **inline** inside
the connection state machine (`state.receive(... &handler)` →
`let response = handler(request)`), then writes the bytes. A single reactor
thread drives all connections, so handlers never overlap.

Two facts make WEB01 tractable rather than a rewrite:

- **`WebApp` is already `Arc<WebApp>` and immutable after construction** (the
  WEB00 open question was resolved in favour of `Arc`). It is `Send + Sync`; the
  handler closure is already `Send + Sync`. So the app can be shared across
  threads today — only the *execution* is serial.
- The lower layers **already provide two parallelism primitives** (built for the
  multi-core TCP work): a sharded multi-reactor runtime and an in-process
  job-queue thread pool. WEB01 is about *wiring*, not inventing.

## The design fork

There are two ways to parallelise, already available beneath `web-core`:

### Path A — sharded reactors (`ShardedTcpRuntime`)

`tcp-runtime` exposes `ShardedTcpRuntime` with `bind_*_sharded(addr, options,
worker_count, handler)` — **N reactor threads**, each owning a subset of
connections, each running the **existing** `Fn(info, &[u8]) -> TcpHandlerResult`
state machine. Connections are distributed across shards (with an explicit
accept fan-out on macOS/BSD, since `SO_REUSEPORT` does not load-balance there —
see the multi-core work).

```
            ┌── reactor shard 0 ── conns {0,4,8,…} ─ inline app.handle ─┐
listener ──▶├── reactor shard 1 ── conns {1,5,9,…} ─ inline app.handle ─┤──▶ responses
            ├── reactor shard 2 ── conns {2,6,…}   ─ inline app.handle ─┤
            └── reactor shard 3 ── conns {3,7,…}   ─ inline app.handle ─┘
```

- **Parallelism unit:** connections. Two requests on *different* connections,
  landing on different shards, run concurrently. Two requests pipelined on the
  *same* connection still serialise (correct for HTTP/1.1 ordering anyway).
- **Handler/response model: UNCHANGED.** The inline `Fn(req)->resp` contract
  stays; the response is produced and written on the same shard thread, so
  HTTP/1.1 response ordering is automatic. No deferred responses.
- **Cost:** `embeddable-http-server` needs a sharded `bind` variant that runs
  its per-connection `HttpConnectionState` across shards; `web-core` adds a
  `bind_*_sharded`/parallel option. `WebApp` is `Arc`-shared across shards.
- **Risk:** low. Reuses shipped infrastructure; no response-model surgery.
- **Limitation:** handler concurrency is capped at `worker_count` reactors and
  is *per connection* — a single client on one keep-alive connection sees no
  speedup (acceptable; that is HTTP/1.1 semantics).

### Path B — in-process mailbox thread pool (`new_inprocess_mailbox`)

`embeddable-tcp-server::new_inprocess_mailbox(... worker_fn)` is a generic
job-queue thread pool: a connection handler **submits** a job to a
`RustThreadPoolJobSubmitter`, a worker runs `worker_fn(JobRequest) ->
JobResult`, and `map_response` writes the result back to the originating
connection. This is the literal "Phase 3" the WEB00 spec describes
(`worker_fn` receives `(JobRequest<HttpRequest>, AppHandle)` and calls
`AppHandle::handle`).

```
reactor ─ parse req ─▶ submit job ─▶ [worker pool: app.handle] ─▶ map_response ─▶ write to conn
   (returns immediately, can serve other connections while workers run)
```

- **Parallelism unit:** requests. Decouples handler concurrency (pool size) from
  the number of I/O threads — even a single reactor can have many in-flight
  handlers.
- **Handler/response model: CHANGED.** The HTTP server must **defer** the
  response: submit the parsed `HttpRequest` as a job and write the
  `HttpResponse` when the worker finishes. This requires:
  - **per-connection response ordering** — on a keep-alive/pipelined HTTP/1.1
    connection, responses MUST be written in request-arrival order even if
    workers finish out of order (a per-connection reorder buffer keyed by a
    monotonic request sequence).
  - **backpressure** — bounded job queue (`worker_queue_depth`); when full, the
    reactor must stop reading that connection (or 503), not unbounded-buffer.
- **Risk:** higher. New deferred-response path in `embeddable-http-server` +
  ordering + backpressure. This is the riskiest change since every port runs on
  this engine.

## Recommendation

**Do Path A first (lower-numbered, lower-risk), keep Path B as a follow-on.**

- **WEB01a — sharded `web-core`.** Reuse `ShardedTcpRuntime`. `web-core` gains a
  parallel/sharded bind that runs the existing inline handler across N reactors;
  `WebApp` stays `Arc`-shared. Delivers real cross-connection parallelism with
  no response-model change. Verified by a CPU-bound benchmark (see below).
- **WEB01b — mailbox pool (optional).** If per-reactor parallelism proves
  insufficient (e.g. fewer reactors than cores, or a need to overlap handlers on
  one connection), add the deferred-response mailbox path. Gated behind WEB01a's
  benchmark data — only build it if the numbers justify the ordering/backpressure
  complexity.

This mirrors how the multi-core TCP arc was sequenced: prove distribution and
scaling with the simpler primitive first, measure, then decide on the heavier
machinery.

## Design concerns (both paths)

- **`WebApp` thread-safety:** already `Arc` + immutable post-construction; hooks
  and routes are read-only at serve time. Confirm no interior mutability in the
  hook registry / settings that needs a lock (audit `app.rs`, `hooks.rs`).
- **Hook semantics:** `before` / `after` / `not_found` / `on_error` run inside
  the same handler invocation, so they execute on the worker/shard thread —
  semantics unchanged. `on_server_start` / `on_server_stop` stay on the binding
  thread.
- **Determinism of tests:** parallel handling makes E2E ordering non-deterministic;
  tests must assert per-request, not global ordering (the existing port E2E
  suites already do — they hit independent routes).
- **macOS accept fan-out:** `SO_REUSEPORT` does not balance on macOS/BSD; Path A
  must use the explicit accept fan-out the sharded runtime already provides.

## Verification (anti-smoke-test)

Echo-on-loopback is **latency-bound, not CPU-bound**, so it will NOT show
throughput scaling (the multi-core lesson: even connection distribution ≠
throughput scaling unless per-request work can saturate a core). WEB01's proof
must use a **CPU-bound handler** (e.g. a fixed busy-loop / hash per request) and
assert that total wall-clock for K concurrent requests drops with `worker_count`
(e.g. K serial-cost requests finish in ≈ K/N × cost with N shards). Add this as
a `web-core` benchmark/integration test, and surface the per-shard distribution,
not just an average.

## Phase / PR plan

1. **WEB01 spec** (this file).
2. **WEB01a-1 — `embeddable-http-server` sharded serve.** A `bind_*_sharded`
   variant that runs the `HttpConnectionState` machine across N reactors. Unit +
   integration tests; the inline handler contract is unchanged.
3. **WEB01a-2 — `web-core` parallel bind.** `WebServer::bind_*_sharded` (or a
   `worker_count` option), `Arc`-sharing the app across shards; CPU-bound
   benchmark proving scaling; per-shard distribution surfaced.
4. **WEB01a-3 — optional per-port opt-in.** Expose `worker_count` through the
   facades that want it (default stays single-reactor for compatibility). Each
   port that opts in adds one executed parallel test.
5. **WEB01b (optional, data-gated) — mailbox pool.** Deferred-response path in
   `embeddable-http-server` + per-connection response ordering + backpressure;
   `web-core` worker_fn = `move |job| app.handle(job.payload)`. Only if WEB01a's
   numbers justify it.

## Open questions (for sign-off before WEB01a-1)

- **Default behaviour:** should parallel serving be opt-in (`worker_count`
  option, default 1 = today's behaviour) or the new default? Recommendation:
  **opt-in** first to avoid changing every port's runtime characteristics at
  once; flip the default once benchmarks + per-port tests are green.
- **Worker count default when opted in:** `cpu_count` vs `cpu_count - 1`
  (leave one core for the acceptor)? Mirror the multi-core bench's choice.
- **Windows (IOCP):** does the sharded runtime cover IOCP, or is Path A
  macOS/Linux-only initially (Windows stays single-reactor)? Confirm
  `ShardedTcpRuntime` platform coverage before WEB01a-1.

## Files (planned)

```
code/specs/WEB01-async-web-core.md                  (this file)
code/packages/rust/embeddable-http-server/          (WEB01a-1: sharded serve)
code/packages/rust/web-core/src/server.rs           (WEB01a-2: parallel bind)
code/packages/rust/web-core/                         (WEB01a-2: CPU-bound benchmark)
```
