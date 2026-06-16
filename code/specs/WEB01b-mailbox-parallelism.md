# WEB01b — Mailbox / deferred-response parallelism for web-core

> **Design spec (specs-first).** No code yet. This scopes WEB01 **Path B**, the
> heavier parallelism path deferred from [WEB01](WEB01-async-web-core.md) pending
> WEB01a's results. WEB01a (sharded reactors) is merged and proven (5.28× on a
> CPU-bound load); WEB01b is taken on next per the user's call. It lays out the
> design, the parts the lower layers already provide, the genuinely hard parts,
> a phased PR plan, and open questions for sign-off **before any engine code**.

## Why WEB01b, given WEB01a already parallelises

WEB01a (`ShardedWebServer`) parallelises **by connection**: N reactor threads,
each running the existing inline handler for its connections. Its ceiling is
`worker_count` and its unit is the connection — two requests pipelined on one
keep-alive connection still serialise, and handler concurrency is pinned to the
number of I/O reactor threads.

WEB01b parallelises **by request**: the I/O thread parses a request and
**submits it as a job** to an in-process worker pool, returning immediately to
service other connections; a worker runs `app.handle` and the response is
written back to the originating connection later. This **decouples handler
concurrency (pool size) from the number of I/O threads** — a single reactor can
have many handlers in flight — and is the model the WEB00 spec's "Phase 3"
originally described (`worker_fn` receives `(JobRequest<HttpRequest>, AppHandle)`).

The two are composable in principle (sharded reactors each feeding a pool) but
WEB01b is specified as a standalone alternative serve mode first.

## What the lower layers already provide (the feasibility finding)

`embeddable-tcp-server::new_inprocess_mailbox(...)` is a generic submit-and-defer
job pool, and a scoping pass confirmed it already has the three hard primitives:

1. **Submit-and-return.** A connection handler gets a `RustThreadPoolJobSubmitter`
   and calls `submit(connection_id, payload)` which enqueues and returns
   immediately ("worker responses are delivered later").
2. **Deferred write-back keyed to the connection.** The worker's `Response` is
   mapped by `map_response(Response) -> TcpMailboxFrame { writes, close }` and the
   framework writes those bytes back to the originating connection out-of-band —
   no change to the receive callback's return value needed.
3. **Per-connection affinity.** Every job is submitted
   `with_affinity_key(connection_id)`, so a given connection's jobs are routed by
   a stable key — the hook on which per-connection response **ordering** hangs
   (see open questions).

So WEB01b is **adapting HTTP framing onto an existing pool**, not building a pool
or out-of-band-write capability from scratch. That materially lowers the risk
the original WEB01 spec assigned to Path B.

## Design

```
reactor (I/O thread)                         worker pool (worker_count threads)
  ─ read bytes ─▶ HttpConnectionState
       buffers until a FULL request is framed
       │  submit(connection_id, HttpRequest)  ─────────▶ worker_fn(JobRequest<HttpRequest>)
       │  (returns immediately; serve next conn)            = app.handle(req) → HttpResponse
       ◀──────────── TcpMailboxFrame (serialized response) ── map_response(HttpResponse)
  ─ write response bytes to that connection
```

- **`embeddable-http-server`** gains a mailbox-backed serve path (a
  `MailboxHttpServer` type, or a serve mode on the existing server). Its
  connection `State` is the existing `HttpConnectionState` (buffer + limits): it
  frames complete requests exactly as today, but instead of calling the handler
  inline it **submits** the parsed `HttpRequest` as a job. `worker_fn` runs the
  `Fn(HttpRequest) -> HttpResponse` handler; `map_response` runs
  `serialize_response` + sets `close`.
- **`web-core`** gains a `MailboxWebServer` (or a `WebServer` mode) whose
  `worker_fn = move |job| app.handle(job.payload)`, sharing one `Arc<WebApp>`
  across the pool (already proven `Send + Sync` + interior-mutability-free in
  WEB01a-2).
- **`conduit`** facade gains a `MailboxServer` (opt-in, like `ShardedServer`).
- Tuning via `EmbeddableTcpServerOptions`: `worker_processes` (pool size),
  `worker_queue_depth` (backpressure bound), `worker_job_timeout`.

## The genuinely hard parts

1. **HTTP/1.1 per-connection response ordering.** On a keep-alive/pipelined
   connection, responses MUST be written in request-arrival order. Workers may
   finish out of order. The affinity key (`connection_id`) routes a connection's
   jobs to a stable worker — IF the pool processes a single affinity key's jobs
   FIFO on one worker, ordering is free. **This must be verified**, not assumed;
   if affinity does not guarantee per-key FIFO, WEB01b needs a per-connection
   reorder buffer keyed by a monotonic request sequence number (hold completed
   responses until all earlier ones on that connection have been written).
2. **Backpressure.** The job queue is bounded (`worker_queue_depth`). When full,
   the reactor must stop reading that connection (or return 503) rather than
   buffer unboundedly — otherwise a slow handler + fast client is a memory-DoS.
3. **Framing across the submit boundary.** A request must be fully framed in the
   connection `State` before submission (partial reads stay buffered per
   connection); only complete `HttpRequest`s become jobs. Pipelined requests in
   one read submit as multiple ordered jobs.
4. **Lifecycle / teardown.** Stopping must drain or cancel in-flight jobs and not
   write to closed connections (the mailbox tracks connection_id → liveness;
   confirm a write to a closed connection is dropped safely).

## Verification (anti-smoke-test)

- **Ordering (deterministic, CI gate):** pipeline N requests on ONE connection
  where handler latency is deliberately inverted (request *i* sleeps `(N-i)·d` so
  later requests finish first); assert responses are still read back in request
  order. This fails loudly if ordering is broken — no flaky timing.
- **Per-request parallelism (deterministic):** the in-flight-gauge technique from
  WEB01a-2 (handlers overlap → max in-flight ≥ 2) but on a SINGLE reactor with a
  pool, proving the decoupling from I/O-thread count.
- **Throughput (`#[ignore]` bench):** CPU-bound handler, compare WEB01a sharded
  vs WEB01b mailbox vs single-reactor; surface the numbers (not a CI gate).

## Phased PR plan

1. **WEB01b spec** (this file).
2. **WEB01b-1 — `embeddable-http-server` mailbox serve** + ordering + backpressure;
   the deterministic ordering and parallelism tests. The riskiest PR.
3. **WEB01b-2 — `web-core` `MailboxWebServer`** (`worker_fn = app.handle`) +
   `conduit` `MailboxServer` (opt-in), with the in-flight-gauge parallelism test
   through the facade.
4. **WEB01b-3 — comparative benchmark** (`#[ignore]`): single-reactor vs sharded
   (WEB01a) vs mailbox (WEB01b) on a CPU-bound load; document when to pick which.

## Open questions (for sign-off before WEB01b-1)

- **Ordering guarantee:** does the `RustThreadPool` process one affinity key's
  jobs FIFO on a single worker? If yes, ordering is free; if no, WEB01b-1 must add
  the per-connection reorder buffer. **This is the first thing WEB01b-1 verifies;
  if it needs the reorder buffer, that materially enlarges WEB01b-1** — flag back
  to the user at that point.
- **Default vs opt-in:** opt-in (a distinct `MailboxServer`/serve mode), default
  unchanged — consistent with WEB01a. Recommended.
- **Compose with WEB01a?** Keep separate initially (mailbox uses a single reactor
  + pool). Sharded-reactors-each-with-a-pool is a later question.
- **Windows/IOCP:** confirm `new_inprocess_mailbox` + the mailbox write-back path
  cover IOCP, or scope WEB01b to kqueue/epoll first.
