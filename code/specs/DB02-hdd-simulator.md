# DB02 — Deterministic HDD Simulator

## Purpose and boundary

This package is the first executable media model in the database and hardware
learning sequence. It models one disk with a deliberately small, explicit
geometry. It teaches address translation, head movement, platter rotation, and
elapsed simulated time. It is not a model of a particular commercial drive.

This layer has no operating system, filesystem, cache, request scheduler,
controller command protocol, failure recovery, or crash durability. Calls are
synchronous: exactly one operation is active, it completes immediately in the
model, and the next call starts from the resulting state. Thus FIFO order is
simply the caller's operation order. It makes no real-time or benchmark claim.

## Fixed geometry and initial state

The v1 profile is `HddGeometryV1`: 2 surfaces, 4 tracks per surface, 4 sectors
per track, and 8 bytes per sector. A sector address is a linear block address
(LBA), in range `[0, 32)`. Mapping is:

```
sectors_per_cylinder = surfaces * sectors_per_track = 8
cylinder = lba // sectors_per_cylinder
surface = (lba // sectors_per_track) % surfaces
sector = lba % sectors_per_track
```

The head's seek coordinate is the cylinder/track number. The selected surface
does not add seek distance. `track_count` is four, and tracks are numbered
0–3. All 32 sectors initially contain eight zero bytes. The head starts at
track 0. The absolute integer clock starts at tick 0; rotational phase starts
at sector 0 and is the absolute tick modulo four. The model has no separate
spindle phase state because it is derived from the clock.

## Timing profile

One tick is an abstract integer unit, not seconds. Seeking costs 3 ticks per
track crossed. Rotation advances continuously during seek and during every
other elapsed interval. After seeking, the drive waits for the requested sector
index to rotate under the head. Transfer takes exactly 1 tick per sector.

For an operation targeting LBA `x`, from state clock `t` and head track `h`:

1. Map the LBA to target track `q = x // 8` and sector phase `s = x % 4`.
2. `seek_ticks = abs(q - h) * 3`.
3. `phase_after_seek = (t + seek_ticks) % 4`.
4. `rotation_ticks = (s - phase_after_seek) % 4`.
5. `transfer_ticks = 1`.
6. Advance clock by the sum. Set head track to `q`.

Every successful read or write returns a frozen timing breakdown containing
`seek_ticks`, `rotation_ticks`, `transfer_ticks`, `total_ticks`, `start_tick`,
and `end_tick`. `total_ticks` is their sum and `end_tick` is
`start_tick + total_ticks`. The final clock is observable in the state snapshot.
A zero-distance seek costs zero; a sector already under the head incurs zero
rotational wait. Each accepted operation still costs its one transfer tick.

## Public behavior

The Python package is `coding-adventures-hdd-simulator`; import package
`hdd_simulator`. It exports `HddGeometryV1`, `HddSimulator`, immutable
`HddState`, `IoTiming`, `SectorResult`, and `HddError` with `InvalidLbaError` and
`InvalidSectorDataError` and `ClockOverflowError` subclasses.

`HddSimulator()` constructs a fresh v1 disk. `read_sector(lba)` returns
`SectorResult(data, timing)` with immutable `bytes` of length eight.
`write_sector(lba, data)` accepts exactly eight bytes and returns
`IoTiming`. `get_state()` returns a frozen snapshot containing current tick,
head track, and all sector bytes in LBA order. The snapshot owns immutable
values and remains unchanged after later operations. There is no reset or
arbitrary geometry constructor in v1; construct a new simulator for a fresh
medium.

The input LBA must be an integer in `[0, 32)`. Python accepts exact built-in
`int` values, excluding booleans and subclasses. Write data is an eight-byte
value. The Python mapping accepts exact built-in `bytes`, `bytearray`, or a live
`memoryview`. The TypeScript mapping accepts plain `Uint8Array` backed by live,
non-shared `ArrayBuffer` storage. Validate the byte count before copying and
copy the payload before mutating the medium. The source must remain stable
for the duration of the call.

The portable maximum tick is 9,007,199,254,740,991. A call whose calculated
end tick exceeds that limit raises `ClockOverflowError`. Validate LBA, then
write payload, then the proposed end tick. Any refusal leaves clock, head,
and media contents unchanged. Errors are deterministic and callers branch on
the error type.

## Invariants

- Clock is an exact integer from zero through the portable maximum and increases
  only after successful I/O.
- Head track is always in 0–3 and changes only after successful I/O.
- The medium always contains exactly 32 sectors of exactly eight bytes each.
- Reads do not mutate media. Writes replace exactly one sector atomically.
- A successful operation's timing total equals its component sum and its end
  tick is the next operation's start tick.
- Rotational phase is always `current_tick % 4`, including while seeking.
- Invalid operations preserve the entire state.

## Conformance vectors

`fixtures/database-media-v1/hdd-simulator.json` is the shared, language-neutral
set of exact state and timing vectors. Implementations should consume these
vectors directly. Supplement them with bounded exhaustive/property-style checks
for all LBAs, all possible start phases, and all head/target track pairs.

## Explicit exclusions and future layers

No claims are made about real HDD geometry, seek curves, RPM, transfer rate,
sector headers, remapping, bad blocks, caches, write barriers, queue depth,
concurrency, power loss, or durability. Those require separately specified
layers. V1 is an idealized functional/timing teaching model with immediate
model-media completion.
