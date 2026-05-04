# Changelog

All notable changes to `executor-protocol` are documented here.  The
format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] — 2026-05-04

Initial release.  Implements spec MX03 V1.

### Added

- **Messages**:
  - `ExecutorRequest` (10 variants): `Register`, `PrepareKernel`,
    `AllocBuffer`, `UploadBuffer`, `Dispatch`, `DownloadBuffer`,
    `FreeBuffer`, `CancelJob`, `Heartbeat`, `Shutdown`
  - `ExecutorResponse` (11 variants): `Registered`, `KernelReady`,
    `BufferAllocated`, `BufferUploaded`, `DispatchDone`, `BufferData`,
    `BufferFreed`, `Cancelled`, `Alive`, `ShuttingDown`, `Error`
  - `ExecutorEvent` (3 variants): `BufferLost`, `ProfileUpdated`,
    `ShuttingDown`
- **Sub-types**:
  - `KernelSource` — 7 variants: `Msl`, `CudaC`, `Glsl`, `SpirV`,
    `Wgsl`, `OpenClC`, `Native` (catch-all)
  - `BackendProfile` — capability bitset + cost model fields
  - `OpTiming { op_index, ns }` — per-op measured timing
  - `ErrorCode { MALFORMED_FRAME, OUT_OF_MEMORY, COMPILATION_FAILED, … }`
    with category bands per spec
- **Frame**: `MessageFrame` with `format_version` + `kind` (Request /
  Response / Event) + `correlation_id` + length-prefixed `payload`
- **Wire format**: hand-rolled binary, identical primitives to
  `matrix-ir` (varint, length-prefixed bytes, tagged unions).
  `MessageFrame::to_bytes()` / `MessageFrame::from_bytes()` round-trip
  deterministically.
- **Transport trait**: `Transport` with async `request(...) -> Result<Response, TransportError>`.
  Async-first so future network transports plug in without
  restructuring.
- **`LocalTransport`**: in-process transport.  In debug builds,
  round-trips every request/response through the wire format to
  enforce the discipline.
- **`block_on`**: hand-rolled minimal `Future` runner (~50 lines).
  Drives `LocalTransport` to completion without a real async runtime.
- **`KernelCacheKey`**: SipHash-based content key (`std::collections::hash_map::DefaultHasher`).
  Includes the variant tag so an MSL kernel and a CUDA kernel with the
  same source text don't collide.

### Security hardening

Same patterns as `matrix-ir` and `compute-ir`:

- All length-prefixed `Vec::with_capacity` calls bound against
  remaining buffer bytes
- `Reader::need` uses `checked_add`
- `bytes()` rejects `u64` lengths exceeding `usize::MAX`
- Varint capped at 10 bytes
- UTF-8 validation on string fields rejects invalid sequences
- Truncation-at-every-byte and 1024-iteration deterministic fuzz tests

### Test coverage: 32 tests passing

- 18 wire round-trip integration tests covering every enum variant
- 1 truncation-at-every-position no-panic test
- 1 forward-compat (future frame version) rejection test
- 1 1024-iteration random-byte fuzz no-panic test
- 3 LocalTransport tests (alloc, dispatch, heartbeat)
- 2 kernel cache tests
- 6 unit tests across modules (varint round-trip, oversized varint,
  message tags unique, error categories, kernel cache key stability,
  block_on with immediate and one-pend futures)

### Constraints

- Zero external dependencies.  Only `matrix-ir` and `compute-ir`
  (path-only).
- The runtime / executor boundary is bytes-on-a-wire only.  No
  closures, trait objects, callbacks, or borrowed references cross
  the line.
