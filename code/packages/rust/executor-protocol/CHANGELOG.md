# Changelog

All notable changes to `executor-protocol` are documented here.  The
format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.2.0] — 2026-05-05

### Added — MX05 Phase 4.1 protocol surface

- New `ExecutorRequest::DispatchSpecialised { job_id, handle, inputs,
  outputs }` variant (wire tag `0x0A`) that routes a previously-emitted
  specialised kernel by **handle**.

  Wire format:
    - `u8 0x0A`
    - `u64 job_id`
    - `u64 handle`
    - `u32 n_inputs` followed by `n_inputs × u64` buffer ids
    - `u32 n_outputs` followed by `n_outputs × u64` buffer ids

  Reply uses the existing `ExecutorResponse::DispatchDone { job_id,
  timings }` on success or `ExecutorResponse::Error { code:
  NOT_IMPLEMENTED, .. }` if the backend hasn't yet wired execution.

- New `ErrorCode::NOT_IMPLEMENTED` (`0x0062`) — soft refusal for
  recognised request shapes the backend doesn't yet execute.
  Distinct from `UNKNOWN_VARIANT`.  Stays in the runtime-defined
  range (< 0x80).

### Changed

- `PROTOCOL_VERSION` bumped from `1` to `2`.  Forward-compatible
  with v1 senders: every existing variant still encodes/decodes
  byte-identically.

### Security hardening

- The `DispatchSpecialised` decoder bounds `Vec::with_capacity` for the
  `inputs` and `outputs` lists against the bytes actually remaining in
  the wire reader (each `BufferId` costs 8 bytes), using the same
  `bounded_capacity` helper that already protects `OpTiming` decoding.
  Without this bound, an attacker-supplied `n_inputs = u32::MAX` would
  request a ~34 GiB allocation per malicious frame.

### Tests (5 new)

- `dispatch_specialised_wire_tag_is_0x0a`
- `not_implemented_error_code_in_runtime_range`
- `dispatch_specialised_request_round_trips`
- `dispatch_specialised_with_empty_buffer_lists_round_trips`
- `dispatch_specialised_oversized_input_count_does_not_oom` — security
  regression test that asserts the decoder rejects a frame claiming
  `n_inputs = u32::MAX` cleanly, without OOM-aborting on the
  pre-allocation.
- `request_tags_unique` extended to include the new variant.

### Notes

- Both `matrix-cpu` and `matrix-metal` reply with `Error { code:
  NOT_IMPLEMENTED }` for `DispatchSpecialised`.  The protocol
  surface lands here; per-backend execution is tracked under
  "MX05 Phase 4.1: matrix-cpu executes specialised kernels" and
  "Phase 4.2: matrix-metal MSL emitter".

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
