# DB02a — C17 Profile for the Deterministic HDD Simulator

This document pins the C17 package for [DB02](DB02-hdd-simulator.md). It
implements the fixed v1 teaching disk and consumes the canonical
`code/specs/fixtures/database-media-v1/hdd-simulator.json` in tests. It does not
change the language-neutral timing model or fixture schema. The package is
`code/packages/c/hdd-simulator`; public declarations are in
`include/hdd_simulator.h`.

## Scope and portability

The library is dependency-free ISO C17 and synchronous. It has no operating
system, filesystem, cache, scheduler, controller protocol, crash recovery,
real-time claim, or persistent storage. It supports exactly the v1 geometry:
2 surfaces, 4 tracks per surface, 4 sectors per track, 8 bytes per sector, 32
linear block addresses (LBAs), and a four-sector rotational phase. Geometry is
not caller-configurable. All dimensions, offsets, byte counts, and tick values
are bounded before use. The implementation requires `uint8_t`, `uint64_t`, and
`size_t` capable of representing the fixed 256-byte medium.

## Public interface

Expose an opaque `HddSimulator` handle, `HddStatus`, fixed constants for sector
count and bytes per sector, an `HddTiming` value, and an `HddSnapshot` value.
All fallible functions return a status; stable status messages are provided by
`hdd_status_message`. Statuses distinguish success, invalid API argument,
invalid LBA, invalid sector-data pointer, wrong sector-data length, clock
overflow, and allocation failure. `hdd_status_message` returns stable,
NUL-terminated text and a stable fallback for unknown enum values.

`hdd_create(HddSimulator **out)` creates the fixed fresh disk. `out` must be a
writable slot not currently owning a live handle; null is invalid. The function
sets a valid output slot to null before allocation, and only stores a handle on
success. `hdd_destroy(NULL)` is a no-op. No public reset, arbitrary-state
constructor, or geometry setter is provided.

`hdd_read_sector(HddSimulator *, size_t lba, uint8_t *out,
size_t out_length, HddTiming *timing_out)` copies exactly eight bytes and
returns exact timing. `hdd_write_sector(HddSimulator *, size_t lba,
const uint8_t *data, size_t data_length, HddTiming *timing_out)` writes exactly
eight copied bytes and returns exact timing. `hdd_get_snapshot(const
HddSimulator *, HddSnapshot *out)` copies the clock, head track, and all 32
sectors in LBA order into caller-owned storage. Snapshot data is detached from
the simulator and remains unchanged after future writes or I/O.

Every pointer argument used as an output must designate writable storage of
the documented type and must not overlap another argument's live storage.
Input pointers must designate readable storage of their explicit length and
remain stable until the function returns. C cannot establish the validity or
extent of an arbitrary non-null pointer. The library retains no caller pointer.
Reads and snapshots do not modify their destination on refusal. Callers may
change or release input data after a successful call.

## Initial state, mapping, and timing

All sector bytes initially equal zero. Initial head track is zero and clock is
zero. For LBA `x`, target track is `x / 8` and target sector is `x % 4`; surface
is `(x / 4) % 2`. Surface does not affect seek distance. Rotational phase is
always `tick % 4`, including while seeking.

For start clock `t`, current head track `h`, target track `q`, and target sector
`s`, calculate in `uint64_t`:

```
seek_ticks      = abs(q - h) * 3
phase_after_seek = ((t % 4) + seek_ticks) % 4
rotation_ticks  = (s + 4 - phase_after_seek) % 4
transfer_ticks  = 1
total_ticks     = seek_ticks + rotation_ticks + transfer_ticks
end_tick        = t + total_ticks
```

Seek is at most nine ticks, rotational wait at most three, and transfer
exactly one, so `total_ticks` is at most 13. Check that the start tick is
within the portable limit and require `total_ticks <= HDD_MAX_TICK - t`
before computing the end. The portable clock
limit is exactly `9,007,199,254,740,991` (`2^53-1`), even on platforms with
larger `uint64_t`. A call ending exactly at this limit succeeds; one ending
above it returns clock overflow. `HddTiming` contains `start_tick`,
`seek_ticks`, `rotation_ticks`, `transfer_ticks`, `total_ticks`, and
`end_tick`. The total is the component sum, and the next successful operation
starts at the prior end tick.

The disk advances only after a successful read or write. Each success sets the
head to the target track and the clock to `end_tick`. A successful write
replaces exactly its target sector. Failed requests leave clock, head, and all
sector bytes unchanged.

## Refusal order and output atomicity

For read and write, validate in this order: non-null simulator handle; LBA in
`[0, 32)`; payload pointer non-null; exact payload length of eight bytes;
required timing output pointer non-null; calculated end tick within the
portable limit. This order makes a collision of invalid conditions produce a
deterministic status. Read's destination is the payload pointer for validation;
write's source is the payload pointer. Snapshot validates handle, then output
pointer. Construction validates output pointer, clears a valid output slot to
null, then attempts allocation. Read, write and snapshot outputs are written
only on success.

For write, after all validation and timing arithmetic pass, copy the eight
input bytes to a fixed local temporary before changing medium state; then
replace the sector and commit head, clock, and timing output. This guarantees
source independence and leaves device state unchanged for all representable
refusals. For read, copy sector bytes and timing only after validation passes,
then commit head and clock. For snapshot, assemble a complete local snapshot
and assign it to the caller only on success. No variable-size allocation is
needed after construction. Pointer aliasing among caller inputs and outputs is
outside the API contract, as is a forged or concurrently modified handle.

## Test adapter and dependency audit

The library and installed/public interface have no runtime dependencies. Tests
may use `code/packages/c/bounded-json` as a test-only dependency after verifying
its contract: bounded input/depth/value/allocation limits; explicit byte input;
owned parse tree; ordered object members; duplicate object keys preserved;
length-aware strings; strict parse of exactly one JSON value; and exact lexical
integer access. The adapter must reject duplicate required keys and validate
every field's type and range; it must not let a JSON boolean pass as an integer,
ignore unknown/missing required fixture structure, or derive expectations from
the simulator under test. The parser's locale-sensitive floating number support
is irrelevant; fixture integers are exact lexical integers.

A fixture-only native adapter reads the canonical JSON from the package's
working directory, bounds file size before allocation, and translates each
case into public API calls. It compares the fixture's named error to the C
status, returned bytes, all timing fields, and clock/head state after every
step. Stable C status messages are checked separately. For refusal vectors,
it compares the before/after snapshot fields and every sector byte, without
depending on C struct padding. `timing_probes`
exercise start clocks and tracks not reachable through ordinary operations;
support these via a test-only seam compiled only into tests (for example, a
test translation unit including a private internal header), never a public
state injection function. Assert that the probe seam is absent from the
installed header and ordinary library build.

`bounded-json` and `iso-harness` are test/build infrastructure only; do not add
either as a runtime dependency or bundle their code into the public library.
Use the ISO C harness for strict C17 builds. Follow its Windows CRT guidance by
keeping any compatibility macro local to the fixture-reader translation unit,
not weakening warnings or changing the library. Run fixture-reading programs
from the package working directory so the relative canonical fixture path is
real and failures to open it cannot become empty test runs.

## Required verification plan

- Unit tests: fresh zero-filled state; each LBA's sector and track mapping;
  exact eight-byte reads/writes; copied source bytes after caller mutation;
  detached snapshots across later writes and timing advances; create/destroy;
  boundary LBAs 0 and 31; stable messages and unknown status fallback.
- Shared trace: consume every canonical case and compare the full operation
  result plus state after each step. Verify all six timing fields, output
  buffers, and unchanged state after every refusal.
- Exhaustive timing: enumerate all 4 head tracks, 4 target tracks, 4 sectors,
  and 4 start phases; independently calculate timing and confirm exact values.
  Include all LBAs across these phases and verify phase advancement through seek,
  rotation, and transfer.
- Clock boundary probes: `2^53-2` with one tick succeeds exactly at the limit;
  `2^53-1` plus a one-tick operation is refused without changing any output or
  device state. Cover calculations with nonzero seek and rotation near the cap.
- Property-style histories: deterministic seeded read/write sequences compared
  with a separate small byte-array/timing model. Assert media size and sector
  width, clock monotonicity, head range, phase relation, timing sum, end/start
  continuity, and invalid-operation state preservation after every step.
- Refusal precedence: collision matrix for null handle, out-of-range LBA,
  null payload, wrong length, null timing output, and overflow. Verify the
  selected status and that every caller output and device field remains
  unchanged on refusal.
- Malformed adapter inputs: truncated JSON, trailing JSON value, wrong schema,
  missing and duplicate keys, wrong types (including boolean LBA), fractional
  and out-of-range integers, invalid hex length/characters, unknown operation,
  malformed timing objects, and values exceeding adapter file/resource caps.
  Confirm failures are diagnostic and never silently skip cases.
- Evidence: run strict ISO C17 builds on every available compiler; ASan and
  UBSan where supported; GCC gcov line/branch evidence (or equivalent compiler
  coverage) with delegate/error branches included; report actual coverage and
  target 95% or more library line coverage. Record compiler/platform
  evidence and do not claim sanitizer or coverage runs that were unavailable.

The checks cover the supported C call contract and the recorded operation
histories. The native suite replays the canonical trace through the exact
DB-E06 source-input route. [Fixture routing](DB-E01-media-fixture-routing.md)
