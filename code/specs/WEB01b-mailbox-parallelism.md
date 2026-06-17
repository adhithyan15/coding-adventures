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

## RESOLVED (scoping pass): ordering is NOT free — reorder buffer required

A source read of the in-process pool (`generic-job-runtime::RustThreadPool`)
settles the ordering open question: its declared capabilities are
**`supports_affinity: false`** and **`supports_ordered_responses: false`**. It is
a **single shared-queue** pool — `try_submit` does `queue.jobs.push_back(job)` and
any idle worker pulls the next job; the `connection_id` affinity key is carried in
metadata but the in-process executor does not pin a key's jobs to one worker, so a
single connection's pipelined requests can run on different workers and **complete
out of order**.

**Consequence:** the *pipelined* path (WEB01b-1b) must add a per-connection
**reorder buffer** (per-connection monotonic sequence; hold completed responses
until all earlier ones on that connection are written). The **WEB01b-1a** slice
sidesteps this entirely by allowing **one in-flight request per connection** (see
below), so it needs no reorder buffer.

## WEB01b-1a — implementation notes (the approved bounded slice)

Scope (user-approved): mailbox-backed serve giving **cross-connection**
parallelism with **one in-flight request per connection** (no intra-connection
pipelining), opt-in, all three platforms. No reorder buffer (deferred to 1b).

Mailbox contract (template: `embeddable-tcp-server` tests around the
`new_inprocess_mailbox` usage — `handle_tcp_bytes_mailbox_with_submitter` +
`map_tcp_output_frame` + a typed `worker_fn`):

- `new_inprocess_mailbox(options, init, handler, on_close, map_response, worker_fn)`.
- `init: Fn(TcpConnectionInfo) -> State` — per-connection state (the HTTP buffer).
- `handler: Fn(info, &mut State, &[u8], &RustThreadPoolJobSubmitter<Req,Resp>) -> TcpHandlerResult`
  — frame a complete `HttpRequest` from the buffer, then `submitter.submit(connection_id, req)`.
- `map_response: Fn(Resp) -> TcpMailboxFrame { writes, close }` — `serialize_response` + close flag.
- `worker_fn: Fn(JobRequest<Req>) -> JobResult<Resp>` — runs the handler (`app.handle`).

**Open question to resolve FIRST in 1a (before coding the read-gating):** after a
worker's response is written back via `map_response`, does the framework re-invoke
the connection `handler` (so the next buffered request can be framed+submitted), or
must the connection be re-armed another way? This determines how "one in-flight per
connection" un-gates. Read the mailbox write-back path / connection bookkeeping in
`embeddable-tcp-server` to confirm before implementing 1a's gating.

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

1. **WEB01b spec** (this file). ✅ merged.
2. **WEB01b-1a — `embeddable-http-server` `MailboxHttpServer`** (the approved
   bounded slice): per-request submit-to-pool + backpressure (503) + the
   deterministic in-flight-gauge parallelism test. ✅ merged (PR #6047). Scope is
   one-request-and-close + *sequential* keep-alive; the original `defer_read`
   read-gating was removed (it *replays* the consumed chunk — see the CI-fix note
   in that PR and `lessons.md`). Pipelined gating/ordering is deferred to 1b.
3. **WEB01b-2 — `web-core` `MailboxWebServer`** (`worker_fn = app.handle`) +
   `conduit` `MailboxServer` (opt-in), with the in-flight-gauge parallelism test
   through the facade. ✅ done (this PR). Single cross-platform `bind`,
   `std::io::Result` (the mailbox stack is `io::Error`-based), `Clone`; mirrors the
   `ShardedWebServer`/`ShardedServer` surface. web-core 0.2.0→0.3.0, conduit
   0.2.0→0.3.0.
4. **WEB01b-1b — per-connection reorder buffer** for the *pipelined* path:
   reassemble the unordered pool's responses into HTTP/1.1 wire order (the pool is
   `supports_ordered_responses = false`). ✅ done. Implemented as an opt-in
   `ordered_responses` flag on `EmbeddableTcpServerOptions` (default off → existing
   consumers byte-identical; `MailboxHttpServer` is the only opt-in): the submitter
   records each connection's job-ids in submission order and the router buffers a
   finished response until every earlier one on that connection has been written.
   No separate in-flight *gate* was needed — the reorder buffer is already bounded
   by the pool's queue depth (a connection that pipelines past it is shed with a
   503). Deterministic tests at both layers:
   `inprocess_mailbox_orders_responses_by_submission_when_enabled`
   (embeddable-tcp-server) and `mailbox_http_server_preserves_pipelined_response_order`
   (embeddable-http-server) — both fail without the buffer.
5. **WEB01b-3 — comparative benchmark** (`#[ignore]`): single-reactor vs sharded
   (WEB01a) vs mailbox (WEB01b) on a CPU-bound load; document when to pick which.
   ✅ done — `web_serving_modes_cpu_bound_comparison` in
   `code/packages/rust/web-core/tests/web_core_test.rs` (web-core 0.3.1). Prints a
   wall-clock + speedup table for all three modes; asserts both parallel modes
   beat the single reactor on ≥ 2 cores. Sample (14 cores): single 180ms, sharded
   5.5×, mailbox 6.6×.

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
