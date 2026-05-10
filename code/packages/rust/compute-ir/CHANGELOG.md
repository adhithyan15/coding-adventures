# Changelog

All notable changes to `compute-ir` are documented here.  The format is
based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] — 2026-05-04

Initial release.  Implements spec MX02 V1.

### Added

- `ExecutorId`, `BufferId`, `KernelId` — placement primitives.
- `CPU_EXECUTOR` constant — `ExecutorId(0)` for the always-available CPU.
- `Residency { executor, buffer }` — where a tensor currently lives.
- `PlacedTensor`, `PlacedConstant` — tensors and constants with
  residency assigned.  Constants may be replicated across executors
  with multiple `PlacedConstant`s sharing a `TensorId`.
- `PlacedOp` — four variants:
  - `Compute` — a normal matrix-ir op with executor + timing
  - `Transfer` — move bytes between residencies
  - `Alloc` — allocate a buffer on an executor
  - `Free` — release a buffer
- `OpTiming { estimated_ns }` — telemetry annotation for inspection.
- `ComputeGraph` — the placed-graph aggregate with `format_version`,
  `inputs`, `outputs`, `constants`, `ops`, `tensors`.
- `ComputeGraph::validate()` — enforces structural and semantic rules:
  format version, tensor table positions, constant byte length,
  per-op executor residency, transfer source matching, alloc/free
  pairing.
- `ComputeGraph::dump()` — human-readable pretty-printer.  Output
  shows executor assignment, transfer endpoints, byte sizes, and
  estimated cost per op in SI units (ns/µs/ms/s) and IEC units
  (B/KiB/MiB/GiB).
- Hand-rolled binary wire format per spec MX03 §"Wire format
  primitives".  `ComputeGraph::to_bytes()` and `ComputeGraph::from_bytes()`
  round-trip deterministically.
- `ComputeIrError` — comprehensive error type covering:
  - Structural: `UndefinedTensor`, `TensorIdMismatch`, `InputOutOfRange`
  - Constant: `ConstantByteLength`
  - Placement: `InputNotResident`, `TransferSourceMismatch`,
    `FreeUnallocated`, `AllocAlreadyAllocated`
  - Wire: `WireUnexpectedEof`, `WireUnsupportedVersion`,
    `WireUnknownTag`, `WireOversizedVarint`, `WireTrailingBytes`
  - Format: `UnsupportedFormatVersion`

### Security hardening

The wire decoder is hardened against malicious input from remote
executors:

- All length-prefixed `Vec::with_capacity` calls bound capacity
  against `remaining_bytes / min_element_bytes` (same pattern used in
  `matrix-ir`).
- `Reader::need` uses `checked_add` to guard 32-bit overflow.
- `bytes()` explicitly rejects `u64` lengths exceeding `usize::MAX`.
- Tests assert no panic across truncation at every byte offset and
  1024 deterministic-PRNG fuzz iterations.

### Test coverage

- 2 hand-built reference graphs (CPU-only neg, CPU↔GPU neg with
  transfers, allocs, frees).
- 7 validator-rejection tests across `TransferSourceMismatch`,
  `InputNotResident`, `AllocAlreadyAllocated`, `FreeUnallocated`,
  `UnsupportedFormatVersion`, `TensorIdMismatch`,
  `ConstantByteLength`.
- 4 wire round-trip tests (CPU-only, CPU↔GPU, with constants, all
  PlacedOp variants) with determinism check.
- 3 decoder hardening tests (amplification, truncation, fuzz).
- 2 `dump()` smoke tests.
- Per-module unit tests for placement primitives, dump formatters,
  wire codec primitives.

### Constraints

- Zero external dependencies.  Only `matrix-ir` (path-only).
- No execution.  Computation happens in the executor crates.
