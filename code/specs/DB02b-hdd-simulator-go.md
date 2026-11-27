# DB02b — Go Profile for the Deterministic HDD Simulator

This document defines the Go profile of [DB02](DB02-hdd-simulator.md). It
preserves the fixed v1 model and shared fixture at
`code/specs/fixtures/database-media-v1/hdd-simulator.json`. The package will be
`code/packages/go/hdd-simulator`, with import package name `hddsimulator` and
no third-party dependencies. All storage is in memory; there is no operating
system, filesystem, cache, queue, controller protocol, persistence, or real-time
claim in the runtime.

## Public API and ownership

The public package exports a fixed-geometry `Device`, `New`, `IOTiming`,
`SectorResult`, `Snapshot`, and sentinel errors:

```go
type Device struct { /* fixed, unexported state */ }

type IOTiming struct {
    StartTick      uint64
    SeekTicks      uint64
    RotationTicks  uint64
    TransferTicks  uint64
    TotalTicks     uint64
    EndTick        uint64
}

type SectorResult struct {
    Data   []byte
    Timing IOTiming
}

type Snapshot struct {
    Tick      uint64
    HeadTrack int
    Sectors   [32][8]byte
}

func New() *Device
func (d *Device) ReadSector(lba int) (SectorResult, error)
func (d *Device) WriteSector(lba int, data []byte) (IOTiming, error)
func (d *Device) Snapshot() (Snapshot, error)
```

The geometry is always 2 surfaces, 4 tracks per surface, 4 sectors per track,
8 bytes per sector, and 32 LBAs. No geometry constructor, setter, reset, or
arbitrary-state constructor is public. `New` returns a fresh zero-filled disk.
The Go zero value of `Device` is also a valid fresh disk because all state is
fixed-size and its natural zero state is the specified initial state. A nil
receiver returns `ErrInvalidArgument`. Separate devices own separate state.

`ReadSector` returns a new, independent eight-byte slice and timing. Mutating
that slice cannot change the medium. `WriteSector` requires a non-nil slice of
exactly eight bytes, copies those bytes into the device, and retains no caller
slice. The caller must keep its input stable for the duration of the call;
concurrent mutation of the same slice while a write reads it is a caller data
race. A nil slice and every wrong length return `ErrInvalidSectorData`.

`Snapshot` returns a detached value containing the tick, head track, and all
sector bytes in LBA order. The fixed array gives value-copy semantics without
nested mutable slices or dependence on JSON encodings. Mutating a snapshot does
not modify the device or another snapshot. Error returns use zero result values;
the API has no caller-provided output pointers to partially modify.

## Addressing, integer bounds, and timing

LBA parameters use Go `int`. Validate `0 <= lba && lba < 32` before converting,
indexing, or deriving offsets. The check uses only constants representable on
both 32-bit and 64-bit Go targets. After validation, the derived track is
`lba / 8`, sector phase is `lba % 4`, and surface is `(lba / 4) % 2`. Track
distance is at most three. Surface does not affect seek cost.

The clock and every timing field use `uint64`. The portable maximum clock is
`9,007,199,254,740,991` (`2^53-1`). One tick is an abstract integer unit. For
current tick `t`, current head `h`, target track `q`, and sector `s`:

```
seek_ticks       = abs(q - h) * 3
phase_after_seek = ((t % 4) + (seek_ticks % 4)) % 4
rotation_ticks   = (s + 4 - phase_after_seek) % 4
transfer_ticks   = 1
total_ticks      = seek_ticks + rotation_ticks + transfer_ticks
end_tick         = t + total_ticks
```

Use the bounded-remainder phase formula shown above; do not add the absolute
clock and seek duration before taking the remainder. Seek is at most nine,
rotation at most three, transfer exactly one, and total elapsed time at most
13. Before adding to the clock, reject an invalid internal tick above the
portable maximum and require `total_ticks <= maxTick - t`; only then calculate
`end_tick`. This makes ordinary `uint64` arithmetic safe and accepts an
operation that ends exactly at the maximum. An operation ending above it
returns `ErrClockOverflow` and changes no state.

On success, the operation commits the target track and end tick. Read does not
change media bytes; write replaces exactly one eight-byte sector. Timing
contains start, seek, rotation, transfer, total, and end ticks; total equals the
component sum, and the following successful call starts at the prior end.
Rotational phase is derived from absolute tick modulo four even while seeking.

## Errors, ordering, and serialization

The API exposes these stable sentinels and messages:

| Sentinel | Message |
|---|---|
| `ErrInvalidArgument` | `invalid API argument` |
| `ErrInvalidLBA` | `LBA out of range` |
| `ErrInvalidSectorData` | `sector data must contain exactly 8 bytes` |
| `ErrClockOverflow` | `clock overflow` |

Callers may use `errors.Is`. For a method call, validate in this order: nil
receiver; LBA range; for writes, non-nil slice and exact length; calculate the
prospective timing and clock limit; then mutate state. Reads allocate and fill
the returned data only after their input and timing checks pass. Writes copy
input into a fixed local `[8]byte` only after nilness/length checks, then commit
the target sector, head, and clock. No error changes device state or returns
partial data/timing. A wrong API value never reaches an index or arithmetic
expression that could overflow.

Calls on a single `Device` are serialized by the caller. The package uses no
mutex and makes no concurrent-access or scheduler guarantee; callers that use
one device from multiple goroutines must protect the whole operation with
their own synchronization. Concurrent operations on different devices are
independent. This mirrors DB02's synchronous one-operation model and keeps
thread scheduling outside the media contract.

## Runtime and fixture boundary

The runtime depends only on the Go standard library, if needed for sentinel
errors, and performs no file, environment, process, or network I/O. Its Go
module declares no external modules. Tests read the canonical JSON fixture from
the package working directory; the JSON decoder and fixture reader are test
only.

The test adapter must bound fixture input before decoding, require the exact
top-level schema and profile, reject duplicate object keys and trailing JSON,
and reject missing, null, unknown, or wrong-typed required values. In Go,
`encoding/json`'s ordinary struct decoding alone is insufficient because it
accepts duplicate keys and missing/null scalars can resemble zero values. Use a
bounded token walk or equivalent duplicate-key check, `json.Decoder` with
`UseNumber` and `DisallowUnknownFields` for typed records, then explicit checks
that each required numeric token is an exact integer and each string, boolean,
array, and object has the required shape. Decode exact signed address tokens
and check host `int` representability before conversion. Pass representable
out-of-range addresses to the device so the trace checks its refusal. The
adapter maps boolean and non-representable addresses to the typed address
refusal without invoking the runtime.

Fixture comparison checks the named error class, every timing field, read
bytes, tick/head after every operation, and media contents through explicit
readback and snapshot assertions. Refusal vectors compare snapshots before
and after. Timing probes use package-internal tests that set the device's
unexported tick/head directly; no state injection method is exported. The
canonical fixture is an oracle; expected values must not be generated by the
device under test.

## Required verification

Package-local tests and repository build fronts cover:

- canonical trace replay, including invalid LBA forms (negative, 32, and JSON
  boolean), the short write payload, exact timing, bytes, and unchanged state;
- unit checks for all 32 addresses, surface/track/sector mapping, fresh zero
  contents, write/readback, copied input, independent read output, snapshots,
  status identity/messages, nil receiver, and wrong lengths;
- a bounded exhaustive timing table over all 4 head tracks, 4 target tracks, 4
  sectors, and 4 start phases (256 timing states), plus every LBA including the
  alternate surface mapping;
- portable-clock boundary probes immediately below and at `2^53-1`, including
  overflowing seek/rotation combinations, with state and returned values
  unchanged on refusal;
- deterministic property-style operation histories against an independent
  fixed byte-array/timing oracle, checking timing sums, phase, head range,
  clock continuity, media contents, and atomic refusals;
- bounded exhaustive short read/write histories over a small number of LBAs
  against the independent oracle;
- `testing.F` fuzz targets for arbitrary `int` LBAs, byte-slice lengths,
  malformed fixture data, and bounded mixed command histories. Cap bytes and
  operations per input; assert refusal atomicity and device invariants after
  every command;
- `go test -count=1 ./...`, `go test -race -count=1 ./...`, `go test -cover
  ./...`, and `go vet ./...`, invoked from the package module directory. Race
  checks include concurrent activity on independently owned devices. Target
  at least 95% statement coverage and record the fuzz campaign budgets.

The package uses `BUILD`, `BUILD_windows`, `README.md`, and `CHANGELOG.md`. The
module and build fronts use the standard toolchain. The declared bounds and
operation schedules define the scope of the manual evidence.

## Local release verification

The independent stepped-clock caller checked 34,048 public commands
and 1,089,536 sector rows. It matched all 22,413 accepted commands
and 11,635 typed refusals. Four controlled runtime mutations caught
rotation calculated before a seek, exact-limit refusal, aliased
read bytes and clock advance on a refused write.
[Fixture Unicode checks](DB-E118-go-hdd-fixture-unicode-scalars.md) · [Fixture model admission](DB-E119-go-hdd-fixture-model-admission.md)
