# generic-job-protocol

Generic cross-language job request and response protocol

This crate defines the protocol seam shared by Rust runtimes and language
bridges:

- `JobRequest<T>` carries a typed unit of work.
- `JobResponse<U>` carries a typed result or portable error.
- `JobMetadata` carries routing, ordering, priority, trace, and affinity facts.
- Metadata helpers validate portable envelopes and compute deadline/attempt facts
  without depending on a runtime scheduler.
- `JobRetryPolicy` classifies retryable errors, exhausted attempts, and capped
  exponential backoff using the same metadata across runtimes.
- `JobResponseSummary` captures terminal status, retryability, trace, and attempt
  facts for runtime read models.
- `JobRequestSummary` and `JobMetadataSummary` expose queue/routing facts without
  requiring runtimes to inspect payload types.
- `JobMetadataBatchSummary` rolls up raw metadata for deadline, priority,
  affinity, sequence, retry, trace, and tag supervision.
- `JobBatchSummary` rolls up request and response summaries for compact
  queue/runtime status views.
- Terminal status and response-summary helpers provide shared read-side
  success, failure, retry, cancel, and timeout classification.
- JSON-line codec helpers provide a phase-one process/stdio wire format.

It intentionally does not implement a thread pool, process pool, TCP runtime, or
FFI binding. Those layers consume this crate and decide how jobs are scheduled.

## JSON-Line Shape

Requests are wrapped in a versioned frame:

```json
{"version":1,"kind":"request","body":{"id":"job-1","payload":{"argv_hex":["50494e47"]},"metadata":{"affinity_key":"7","sequence":1}}}
```

Responses use the same envelope:

```json
{"version":1,"kind":"response","body":{"id":"job-1","result":{"status":"ok","payload":{"resp_hex":"2b504f4e470d0a"}},"metadata":{"affinity_key":"7","sequence":1}}}
```

Payloads remain generic. A TCP Redis adapter can use `argv_hex`/`resp_hex`, while
future image, parser, compression, or FFI adapters can use their own payload
types without changing the job envelope.

## Development

```bash
# Run tests
bash BUILD
```
