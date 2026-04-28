# JR00 - Generic Job Runtime

## Overview

`generic-job-runtime` is a language-neutral contract for distributing units of
work across an execution backend.

The first implementation slice is `generic-job-protocol`, a Rust crate that
defines the portable `JobRequest<T>` / `JobResponse<U>` envelope and JSON-line
wire codec. It is deliberately separate from any TCP runtime, worker pool, FFI
bridge, or scheduler implementation. Those layers consume the protocol crate.

The core idea is simple:

```text
producer creates JobRequest<T>
executor runs the work wherever it is safe and efficient
consumer receives JobResponse<U>
```

The executor may be:

- an inline executor for tests
- a Rust thread pool
- a Python, Ruby, Perl, or Node process pool
- a language-native thread pool when the language can run callbacks in parallel
- a remote worker pool in a future distributed runtime

The abstraction must not be TCP-specific. TCP servers are one important adapter,
but the same job model should also support CPU-bound work such as parsing,
compression, image tiling, layout, query execution, and ML preprocessing.

## Why This Exists

The repository is building high-performance native runtimes that should be
usable from many languages.

For TCP servers, the Rust layer can own:

- sockets
- native event loops
- buffers
- backpressure
- timers
- completion routing

But user work may need to run elsewhere:

- in Rust threads for native hot paths
- in Python worker processes to avoid one global interpreter lock
- in Ruby worker processes to avoid one global VM lock
- in Perl worker processes when interpreter isolation is simpler than shared
  embedded callbacks
- in a language-native thread pool for runtimes that support parallel execution

The runtime therefore needs a small, generic job contract that lets producers
submit typed requests and receive typed responses without knowing how the work
was executed.

## Non-Goals

`generic-job-runtime` is not:

- an operating-system scheduler like `os-job-core`
- a cron or recurring-task framework
- a TCP protocol framework
- a replacement for Rayon inside pure Rust CPU kernels
- a distributed task queue such as Celery, Sidekiq, or Resque
- a promise that all languages have the same concurrency model

It is the common execution seam that those higher-level systems can target.

## Layering

```text
Application code
  - Mini Redis command execution
  - IRC message handling
  - image tile rendering
  - parser work units
  - compression chunks
    |
    v
language package API
  - Python, Ruby, Perl, TypeScript, Go, Rust, ...
    |
    v
generic-job-runtime
  - JobRequest<T>
  - JobResponse<U>
  - JobExecutor<T, U>
  - ordering, backpressure, cancellation, timeouts
    |
    +--> inline executor
    +--> thread-pool executor
    +--> process-pool executor
    +--> future remote executor
```

TCP integration is an adapter above this core:

```text
tcp-job-runtime
  turns readable socket bytes into JobRequest<TcpWork>
  turns JobResponse<TcpAction> into writes, closes, or no-ops
    |
    v
generic-job-runtime
    |
    v
tcp-runtime
    |
    v
stream-reactor
    |
    v
transport-platform
```

## Core Concepts

### Job Identity

Every job has a stable id.

```text
JobId
  opaque string or integer
```

Ids exist so that:

- responses can be matched to requests
- cancellation can target a submitted job
- timeouts can identify the failed unit of work
- metrics can attribute latency and failures

Ids must be unique within one executor instance. They do not need to be globally
unique across machines in phase one.

### JobRequest<T>

`JobRequest<T>` is the generic unit of work.

```text
JobRequest<T>
  id: JobId
  payload: T
  metadata: JobMetadata
```

`T` is language-specific in pure in-process implementations. Across a process
or FFI boundary, `T` must be encoded into a portable wire representation.

### JobResponse<U>

`JobResponse<U>` is the result of one submitted job.

```text
JobResponse<U>
  id: JobId
  result: JobResult<U>
  metadata: JobMetadata
```

```text
JobResult<U>
  ok(U)
  error(JobError)
  cancelled(JobCancellation)
  timed_out(JobTimeout)
```

`U` is the typed success value. Failure is represented by a portable error
shape, not by language-specific exceptions crossing runtime boundaries.

### JobMetadata

Metadata carries scheduling and routing facts without polluting the payload.

```text
JobMetadata
  created_at_ms
  deadline_at_ms?
  priority
  affinity_key?
  sequence?
  attempt
  trace_id?
  tags
```

Important fields:

- `deadline_at_ms` lets executors reject or cancel stale work.
- `priority` lets latency-sensitive work jump ahead of background work.
- `affinity_key` keeps related jobs on the same worker when useful.
- `sequence` preserves producer ordering when responses must be applied in
  request order.
- `attempt` supports retry accounting.
- `trace_id` connects executor metrics back to application traces.

### Affinity

Affinity is how the generic runtime supports protocols such as TCP without
becoming protocol-specific.

For TCP:

```text
affinity_key = connection_id
sequence = per_connection_sequence_number
```

For image rendering:

```text
affinity_key = image_id
sequence = tile_index
```

For parsing:

```text
affinity_key = document_id
sequence = chunk_index
```

The job runtime should support multiple ordering policies:

```text
OrderingPolicy
  unordered
  ordered_by_affinity
  ordered_globally
```

Most high-performance systems should prefer `ordered_by_affinity`, because it
preserves per-connection or per-document order without serializing unrelated
work.

## Executor Interface

The conceptual API is:

```text
JobExecutor<T, U>
  capabilities() -> ExecutorCapabilities
  submit(JobRequest<T>) -> SubmitResult
  try_submit(JobRequest<T>) -> SubmitResult
  drain_responses(max) -> Vec<JobResponse<U>>
  cancel(JobId) -> CancelResult
  shutdown(ShutdownMode) -> ShutdownResult
```

The exact syntax differs by language, but the behavior must match.

### SubmitResult

```text
SubmitResult
  accepted
  rejected(SubmitError)
```

```text
SubmitError
  queue_full
  shutting_down
  unsupported_payload
  deadline_already_expired
  worker_unavailable
  codec_error
```

The producer must treat `queue_full` as a backpressure signal, not as a crash.

### ExecutorCapabilities

Capabilities tell adapters how to use an executor safely.

```text
ExecutorCapabilities
  supports_parallel_execution
  supports_parallel_callbacks
  requires_vm_lock
  supports_process_isolation
  supports_cancellation
  supports_timeouts
  supports_affinity
  supports_ordered_responses
  requires_serializable_payloads
  max_workers
  max_queue_depth
  max_payload_bytes
```

Examples:

```text
Rust thread pool
  supports_parallel_execution = true
  supports_parallel_callbacks = true
  requires_vm_lock = false
  supports_process_isolation = false
  requires_serializable_payloads = false
```

```text
Python process pool
  supports_parallel_execution = true
  supports_parallel_callbacks = true
  requires_vm_lock = false for the parent runtime
  supports_process_isolation = true
  requires_serializable_payloads = true
```

```text
Embedded Python callback on one interpreter
  supports_parallel_execution = false for Python code
  supports_parallel_callbacks = false
  requires_vm_lock = true
  supports_process_isolation = false
```

## Executor Types

### Inline Executor

Runs work immediately on the caller thread.

Use cases:

- unit tests
- deterministic examples
- tiny programs where concurrency is not needed

This executor must be correct, but it is not the performance path.

### Thread-Pool Executor

Runs work on an in-process worker pool.

Use cases:

- Rust CPU-bound work
- Go work scheduled by goroutines and worker pools
- JVM or CLR work where parallel callbacks are safe
- any native extension path that keeps hot work in Rust

Thread-pool executors must expose:

- worker count
- queue depth
- panic or exception containment
- timeout behavior
- shutdown semantics

Rust status: `generic-job-runtime` now includes a transport-neutral
`RustThreadPool<T, U>` executor. The pool accepts only `JobRequest<T>` values,
returns only `JobResponse<U>` values, and has no view into TCP, Redis, IRC, UI
events, or any future producer.

### Process-Pool Executor

Runs work in child processes.

Use cases:

- Python application callbacks that need parallel CPU execution
- Ruby callbacks that need parallel CPU execution
- Perl callbacks where interpreter isolation is simpler and safer
- crash isolation for untrusted or experimental work

Process-pool executors must expose:

- worker command or module entrypoint
- serialization codec
- worker startup timeout
- per-job timeout
- max in-flight jobs per worker
- restart policy
- crash handling
- stderr/stdout handling

The parent process must never allow one dead worker to stall the completion
queue forever.

### Remote Executor

Remote execution is future work.

The phase-one API should not require network distribution, but the request and
response model should be explicit enough that a future remote executor can use
the same concepts.

## Serialization

In-process executors can pass native values directly.

Process and remote executors need codecs.

```text
JobCodec<T, U>
  encode_request(JobRequest<T>) -> bytes
  decode_request(bytes) -> JobRequest<T>
  encode_response(JobResponse<U>) -> bytes
  decode_response(bytes) -> JobResponse<U>
```

Codec requirements:

- reject malformed frames
- reject oversized frames
- reject unknown enum values
- preserve job ids exactly
- preserve metadata needed for ordering and tracing
- fail closed on short, padded, or inconsistent payloads

Recommended phase-one codecs:

- JSON lines for easiest cross-language implementation
- length-prefixed binary frames for higher throughput once the contract is
  stable

The wire format must be versioned:

```text
JobWireFrame
  version
  kind: request | response | heartbeat | shutdown
  length
  payload
```

## Backpressure

Backpressure is a first-class part of the contract.

An executor has bounded queues:

```text
ExecutorLimits
  max_queue_depth
  max_in_flight
  max_payload_bytes
  max_response_bytes
  max_worker_restarts_per_minute
```

When limits are reached:

- `try_submit` returns `queue_full`
- adapters pause reads, stop accepting work, or shed load
- metrics record the rejection
- no unbounded memory growth is allowed

For TCP, this means:

```text
worker queue full
  -> stop reading from affected connection
  -> keep socket registered for write/close events as needed
  -> resume reads when completion or queue capacity returns
```

## Cancellation And Timeouts

Every executor must define what cancellation means.

```text
CancellationState
  not_started
  running
  already_completed
  cancelled
  unsupported
```

Thread-pool cancellation may only cancel jobs that have not started.
Process-pool cancellation may kill or recycle a worker if the job cannot be
interrupted safely.

Timeouts are mandatory for process-backed language workers. A stuck Python or
Ruby job must not stall the Rust reactor, the parent runtime, or unrelated
jobs.

## Error Model

Errors must be portable.

```text
JobError
  code
  message
  retryable
  origin
  detail
```

`origin` should distinguish:

- producer
- executor
- worker
- codec
- timeout
- cancellation
- panic_or_exception

Language-specific exceptions are converted into `JobError`; they do not cross
FFI or process boundaries as raw objects.

## Cross-Language API Shape

Every language implementation should expose the same concepts, adapted to local
idioms.

### Rust

```rust
pub struct JobRequest<T> {
    pub id: JobId,
    pub payload: T,
    pub metadata: JobMetadata,
}

pub struct JobResponse<U> {
    pub id: JobId,
    pub result: Result<U, JobError>,
    pub metadata: JobMetadata,
}

pub trait JobExecutor<T, U> {
    fn capabilities(&self) -> ExecutorCapabilities;
    fn submit(&self, request: JobRequest<T>) -> Result<(), SubmitError>;
    fn drain_responses(&self, max: usize) -> Vec<JobResponse<U>>;
    fn cancel(&self, id: JobId) -> CancelResult;
}
```

### Python

```python
request = JobRequest(
    id=job_id,
    payload=payload,
    metadata=metadata,
)

executor.submit(request)
responses = executor.drain_responses(max=128)
```

Python packages should support both:

- pure Python worker-process execution
- native Rust-backed executors through extension modules

For CPU-bound Python callbacks, the default high-performance backend should be a
process pool, not a single embedded callback guarded by the GIL.

### Ruby

```ruby
request = JobRequest.new(
  id: job_id,
  payload: payload,
  metadata: metadata
)

executor.submit(request)
responses = executor.drain_responses(max: 128)
```

For CPU-bound Ruby callbacks, the default high-performance backend should be a
process pool, not a single embedded callback guarded by the GVL.

### Perl

```perl
my $request = JobRequest->new(
    id => $job_id,
    payload => $payload,
    metadata => $metadata,
);

$executor->submit($request);
my @responses = $executor->drain_responses(max => 128);
```

Perl implementations may start with process workers and add native extensions
later.

### Go

```go
type JobRequest[T any] struct {
    ID       JobID
    Payload  T
    Metadata JobMetadata
}

type JobExecutor[T any, U any] interface {
    Submit(JobRequest[T]) error
    DrainResponses(max int) []JobResponse[U]
}
```

Go can map executors naturally onto goroutines and bounded channels.

### TypeScript

```typescript
type JobRequest<T> = {
  id: JobId;
  payload: T;
  metadata: JobMetadata;
};

type JobExecutor<T, U> = {
  submit(request: JobRequest<T>): Promise<void>;
  drainResponses(max: number): Promise<JobResponse<U>[]>;
};
```

Node implementations may use worker threads, child processes, or a native
extension-backed executor depending on the workload.

## TCP Adapter

The TCP adapter should not change the generic core.

It defines TCP-specific payload and output types:

```text
TcpJobRequest
  connection_id
  sequence
  bytes
  peer_addr
  local_addr
```

```text
TcpJobResponse
  connection_id
  sequence
  action
```

```text
TcpAction
  write(bytes)
  write_and_close(bytes)
  close
  no_op
  fail_and_close(error)
```

The adapter maps:

```text
socket readable -> JobRequest<TcpJobRequest>
JobResponse<TcpJobResponse> -> socket write/close/no-op
```

The adapter must preserve per-connection ordering unless the application opts
into unordered responses.

The adapter must use executor backpressure to pause reads. It must not keep
reading unlimited bytes from sockets while workers are saturated.

## Relationship To Existing Specs

### `tcp-runtime`

`tcp-runtime` owns socket policy and byte-stream progression. It should remain
usable with simple inline callbacks.

`tcp-job-runtime` is a future adapter that uses `generic-job-runtime` to keep
application work off the reactor thread.

### `D18C-chief-of-staff-job-framework`

Chief of Staff jobs are scheduled tasks: reminders, digests, indexing, and
agent runs.

`generic-job-runtime` jobs are execution work units. They may live for
milliseconds and exist only in memory.

The two specs share vocabulary, but solve different problems.

### `DS01-ffi-bridges`

FFI bridges expose native packages to host languages.

For `generic-job-runtime`, FFI surfaces should expose opaque handles and stable
wire frames. They should not expose Rust generics directly over C ABI.

## FFI Boundary

Rust generics do not cross C ABI boundaries. Native extension crates must expose
opaque runtime handles and byte-oriented submission APIs.

Good FFI shape:

```text
job_runtime_new(options_json) -> handle
job_runtime_free(handle)
job_runtime_submit(handle, request_bytes, request_len) -> status
job_runtime_drain(handle, out_buffer, out_len) -> bytes_written
job_runtime_cancel(handle, job_id_bytes, job_id_len) -> status
```

Bad FFI shape:

- exposing Rust enum layouts directly
- exposing raw pointers into executor-owned queues
- requiring Python, Ruby, or Perl to understand Rust lifetimes
- letting Rust panics cross the FFI boundary
- calling host-language callbacks while holding internal executor locks

Caller-controlled enum-like values must be represented as integers or strings
and validated explicitly before use.

## Security And Resource Rules

The job runtime is a trust boundary whenever it accepts work from another
language, process, or network-facing adapter.

Required protections:

- bounded request size
- bounded response size
- bounded queue depth
- bounded worker count
- per-job timeout
- worker crash recovery
- malformed frame rejection
- panic and exception containment
- no unbounded retries
- no raw language objects across process boundaries
- no raw Rust references across FFI boundaries

Process-backed executors must treat worker stdout/stderr as untrusted data.

## Metrics

Every implementation should expose:

```text
submitted_total
accepted_total
rejected_total
completed_total
failed_total
cancelled_total
timed_out_total
queue_depth
in_flight
worker_count
worker_restarts
latency_ms
execution_ms
serialization_ms
```

These metrics are essential for deciding whether bottlenecks live in:

- producer input rate
- executor queueing
- worker execution
- serialization
- completion application

## Testing Strategy

Each language implementation should share behavioral tests:

- submit one job and receive one response
- submit many jobs and receive all responses
- reject work when queue depth is exceeded
- preserve order under `ordered_by_affinity`
- allow unrelated affinity groups to complete independently
- cancel a not-yet-started job
- time out a stuck job
- convert worker exceptions into `JobError`
- survive one worker crash in process-pool mode
- reject malformed serialized requests
- reject oversized requests and responses

TCP adapter tests should additionally verify:

- socket reads become job requests
- job responses become socket writes
- per-connection ordering is preserved
- executor backpressure pauses reads
- failed jobs close or error the correct connection

## Phase Plan

### Phase 1: Core Contract

Add the pure data model in every target language:

- `JobId`
- `JobRequest<T>`
- `JobResponse<U>`
- `JobMetadata`
- `JobError`
- `ExecutorCapabilities`
- validation helpers

Rust should land first because the TCP runtime will consume it.

### Phase 2: Stdio Process-Pool Executor

Status: Rust slices have landed with bounded in-flight submission, affinity
routing, async response draining, per-job timeout responses, worker-exit
failure responses, opt-in worker restart policy, queue-full backpressure, and a
TCP consumer in `embeddable-tcp-server` that pauses and replays TCP reads when
worker capacity is saturated.

Add a language-neutral process-pool executor using JSON lines first so Python,
Ruby, Perl, Lua, and other process-backed hosts can share the same execution
seam. This phase proves that CPU-bound callbacks can run outside one GIL or VM
lock.

Remaining hardening:

- worker startup timeouts
- cancellation semantics
- ordered response buffering by affinity where adapters need it
- metrics for queue pressure, paused reads, replay counts, and worker
  saturation

### Phase 3: Rust Thread-Pool Executor

Status: initial Rust implementation has landed with:

- bounded queue
- worker count
- response draining
- panic containment
- timeout accounting
- queued-job cancellation and logical running-job cancellation

Remaining hardening:

- affinity-aware ordering
- cooperative cancellation tokens for long-running CPU jobs
- metrics for queue depth, in-flight jobs, cancellations, and panics

### Phase 4: TCP Job Runtime Adapter

Add a TCP adapter that proves:

- the reactor can submit work without executing it inline
- completions route back to the correct socket
- backpressure protects the reactor when workers saturate

Mini Redis and IRC can then choose between inline execution and job-backed
execution.

### Phase 5: Language Bindings

Expose package APIs in:

- Python
- Ruby
- Perl
- Go
- TypeScript
- Rust

Each package should preserve the same behavior even when the local syntax
differs.

### Phase 6: Benchmarking

Use the benchmark tool to compare:

- inline executor
- Rust thread-pool executor
- Python process-pool executor
- Ruby process-pool executor
- TCP inline callback
- TCP job-backed callback

Benchmarks must report latency, throughput, queue depth, CPU use, worker
restarts, and serialization overhead.

## Open Questions

- Should phase one use JSON lines or a small custom binary frame?
- Should ordering be implemented inside executors or in adapters that drain
  responses?
- Should process pools use stdin/stdout, Unix sockets, named pipes, or TCP
  loopback for worker transport?
- Should retries be generic, or should adapters own retry policy?
- Should `JobMetadata` support structured capability requirements per job?
- Should worker processes be long-lived by default, or should some language
  bindings support one-shot isolation for untrusted jobs?

## First Implementation Target

The first implementation should be small:

```text
code/packages/rust/job-runtime-core
code/packages/rust/job-executor-thread-pool
```

Only after that should TCP integration begin:

```text
code/packages/rust/tcp-job-runtime
```

This keeps the core abstraction honest. If `job-runtime-core` cannot be useful
for non-TCP CPU work, the design is too narrow.
